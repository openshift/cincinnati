//! Cincinnati backend: policy-engine server.

#![deny(missing_docs)]

#[macro_use]
extern crate cincinnati;
#[macro_use]
extern crate commons;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate smart_default;
#[macro_use]
extern crate structopt;
#[macro_use]
extern crate custom_debug_derive;

mod config;
mod graph;
mod openapi;
mod status;

use actix_cors::Cors;
use actix_service::Service;
use actix_web::http::StatusCode;
use actix_web::{http, middleware, App, HttpRequest, HttpResponse, HttpServer};
use cincinnati::plugins::BoxedPlugin;
use commons::prelude_errors::*;
use commons::tracing::{get_tracer, init_tracer, set_span_tags};
use commons::{
    format_request,
    metrics::{self, HasRegistry},
};
use futures::future;
use opentelemetry::{
    trace::{mark_span_as_active, FutureExt, Tracer},
    Context as ot_context,
};
use parking_lot::RwLock;
use prometheus::{labels, opts, Counter, Registry};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
/// Build info
mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

/// Common prefix for policy-engine metrics.
pub static METRICS_PREFIX: &str = "cincinnati_pe";

lazy_static! {
    static ref BUILD_INFO: Counter = Counter::with_opts(opts!(
        "build_info",
        "Build information",
        labels! {
            "git_commit" => built_info::GIT_COMMIT_HASH.unwrap_or("unknown"),
        }
    ))
    .unwrap();
}

#[actix_web::main]
async fn main() -> Result<(), Error> {
    let settings = config::AppSettings::assemble()?;
    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), settings.verbosity)
        .filter(Some("cincinnati"), settings.verbosity)
        .init();
    info!("application settings:\n{:#?}", &settings);

    // Metrics service.
    let registry: &'static Registry = Box::leak(Box::new(metrics::new_registry(Some(
        METRICS_PREFIX.to_string(),
    ))?));
    registry.register(Box::new(BUILD_INFO.clone()))?;

    // Main service.
    let plugins = settings.validate_and_build_plugins(Some(registry))?;

    // Shared state.
    let state = {
        let mandatory_params = settings.mandatory_client_parameters.clone();
        let path_prefix = settings.path_prefix.clone();
        let plugins = Box::leak(Box::new(plugins));
        let live = Arc::new(RwLock::new(false));
        let ready = Arc::new(RwLock::new(false));

        AppState::new(
            mandatory_params,
            path_prefix,
            plugins,
            live,
            ready,
            registry,
        )
    };

    graph::register_metrics(state.registry())?;
    let metric_state = state.clone();
    let metrics_server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .app_data(actix_web::web::Data::new(metric_state.clone()))
            .service(
                actix_web::web::resource("/metrics")
                    .route(actix_web::web::get().to(metrics::serve::<AppState>)),
            )
            .service(
                actix_web::web::resource("/livez")
                    .route(actix_web::web::get().to(status::serve_liveness)),
            )
            .service(
                actix_web::web::resource("/readyz")
                    .route(actix_web::web::get().to(status::serve_readiness)),
            )
    })
    .bind((settings.status_address, settings.status_port))?
    .run();

    // Enable tracing
    init_tracer("policy-engine", settings.tracing_endpoint.clone())?;
    let main_state = state.clone();
    let main_server = HttpServer::new(move || {
        let app_prefix = main_state.path_prefix.clone();
        App::new()
            .wrap_fn(|req, srv| {
                let mut span = get_tracer().start("request");
                set_span_tags(req.path(), req.headers(), &mut span);
                let _active_span = mark_span_as_active(span);
                let cx = ot_context::current();
                srv.call(req).with_context(cx)
            })
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["HEAD", "GET"]),
            )
            .app_data(actix_web::web::Data::<AppState>::new(main_state.clone()))
            .service(
                // keeping this for backward compatibility
                actix_web::web::resource(&format!("{}/v1/graph", app_prefix))
                    .route(actix_web::web::get().to(graph::index)),
            )
            .service(
                actix_web::web::resource(&format!("{}/graph", app_prefix))
                    .route(actix_web::web::get().to(graph::index)),
            )
            .service(
                actix_web::web::resource(&format!("{}/openapi", app_prefix))
                    .route(actix_web::web::get().to(openapi::index)),
            )
            .service(
                actix_web::web::resource(&format!("{}/v1/openapi", app_prefix))
                    .route(actix_web::web::get().to(openapi::index)),
            )
            .default_service(actix_web::web::route().to(default_response))
    })
    .backlog(settings.backlog)
    .max_connections(settings.max_connections)
    .max_connection_rate(settings.max_connection_rate)
    .keep_alive(settings.keep_alive)
    .client_request_timeout(settings.client_timeout)
    .bind((settings.address, settings.port))?
    .run();

    // metrics endpoints has started running
    *state.live.write() = true;

    let http_req = actix_web::test::TestRequest::get()
        .uri(&format!(
            "{}?channel=stable-4.10",
            "http://ready.probe/graph"
        ))
        .insert_header((
            http::header::ACCEPT,
            http::header::HeaderValue::from_static(cincinnati::CONTENT_TYPE),
        ))
        .to_http_request();

    info!("waiting for the application to be ready");

    // wait for the application to be initialized and the cache refreshed.
    while *state.ready.read() == false {
        thread::sleep(Duration::new(10, 0));
        let resp = graph::index(
            http_req.clone(),
            actix_web::web::Data::<AppState>::new(state.clone()),
        )
        .await;
        let status =
            resp.unwrap_or_else(|err| HttpResponse::InternalServerError().body(err.to_string()));
        if status.status().is_success() {
            info!("application is ready");
            *state.ready.write() = true;
        }
    }

    BUILD_INFO.inc();
    future::try_join(metrics_server, main_server).await?;
    Ok(())
}

// log errors in case an incorrect endpoint is called
async fn default_response(req: HttpRequest) -> HttpResponse {
    error!(
        "Error serving request '{}' from '{}': Incorrect Endpoint",
        format_request(&req),
        req.peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "<not available>".into())
    );
    HttpResponse::new(StatusCode::NOT_FOUND)
}

/// Shared application configuration (cloned per-thread).
#[derive(Clone, Debug)]
pub struct AppState {
    /// Query parameters that must be present in all client requests.
    mandatory_params: HashSet<String>,
    /// Upstream cincinnati service.
    path_prefix: String,
    /// Policy plugins.
    plugins: &'static [BoxedPlugin],
    live: Arc<RwLock<bool>>,
    ready: Arc<RwLock<bool>>,
    registry: &'static Registry,
}

impl AppState {
    /// Creates a new State with the given arguments
    pub fn new(
        mandatory_params: HashSet<String>,
        path_prefix: String,
        plugins: &'static [BoxedPlugin],
        live: Arc<RwLock<bool>>,
        ready: Arc<RwLock<bool>>,
        registry: &'static Registry,
    ) -> AppState {
        AppState {
            mandatory_params,
            path_prefix,
            plugins,
            live,
            ready,
            registry,
        }
    }

    /// Returns the boolean inside self.live
    pub fn is_live(&self) -> bool {
        *self.live.read()
    }

    /// Returns the boolean inside self.ready
    pub fn is_ready(&self) -> bool {
        *self.ready.read()
    }
}

impl Default for AppState {
    fn default() -> Self {
        let registry: &'static Registry = Box::leak(Box::new(
            metrics::new_registry(Some(METRICS_PREFIX.to_string())).unwrap(),
        ));
        AppState {
            mandatory_params: Default::default(),
            path_prefix: Default::default(),
            plugins: Default::default(),
            live: Default::default(),
            ready: Default::default(),
            registry,
        }
    }
}

impl HasRegistry for AppState {
    fn registry(&self) -> &'static Registry {
        self.registry
    }
}
