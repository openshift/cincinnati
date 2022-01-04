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

use actix_cors::Cors;
use actix_service::Service;
use actix_web::http::StatusCode;
use actix_web::{middleware, App, HttpRequest, HttpResponse, HttpServer};
use cincinnati::plugins::BoxedPlugin;
use commons::metrics::{self, RegistryWrapper};
use commons::prelude_errors::*;
use commons::tracing::{get_tracer, init_tracer, set_span_tags};
use futures::future;
use opentelemetry::{
    trace::{mark_span_as_active, FutureExt, Tracer},
    Context as ot_context,
};
use prometheus::{labels, opts, Counter, Registry};
use std::collections::HashSet;
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
    debug!("application settings:\n{:#?}", &settings);

    // Metrics service.
    let registry: &'static Registry = Box::leak(Box::new(metrics::new_registry(Some(
        METRICS_PREFIX.to_string(),
    ))?));
    graph::register_metrics(registry)?;
    registry.register(Box::new(BUILD_INFO.clone()))?;
    let metrics_server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .app_data(actix_web::web::Data::new(RegistryWrapper(registry)))
            .service(
                actix_web::web::resource("/metrics")
                    .route(actix_web::web::get().to(metrics::serve::<RegistryWrapper>)),
            )
    })
    .keep_alive(Duration::new(30,0))
    .bind((settings.status_address, settings.status_port))?
    .run();

    // Enable tracing
    init_tracer("policy-engine", settings.tracing_endpoint.clone())?;

    // Main service.
    let plugins = settings.validate_and_build_plugins(Some(registry))?;
    let state = AppState {
        mandatory_params: settings.mandatory_client_parameters.clone(),
        path_prefix: settings.path_prefix.clone(),
        plugins: Box::leak(Box::new(plugins)),
    };

    let main_server = HttpServer::new(move || {
        let app_prefix = state.path_prefix.clone();
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
            .app_data(actix_web::web::Data::<AppState>::new(state.clone()))
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
            .default_service(actix_web::web::route().to(default_response))
    })
    .backlog(settings.backlog)
    .max_connections(settings.max_connections)
    .max_connection_rate(settings.max_connection_rate)
    .keep_alive(settings.keep_alive)
    .client_request_timeout(settings.client_timeout)
    .bind((settings.address, settings.port))?
    .run();

    BUILD_INFO.inc();

    future::try_join(metrics_server, main_server).await?;
    Ok(())
}

// log errors in case an incorrect endpoint is called
async fn default_response(req: HttpRequest) -> HttpResponse {
    error!(
        "Error serving request '{}' from '{}': Incorrect Endpoint",
        graph::format_request(&req),
        req.peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "<not available>".into())
    );
    HttpResponse::new(StatusCode::NOT_FOUND)
}

/// Shared application configuration (cloned per-thread).
#[derive(Clone, Debug)]
struct AppState {
    /// Query parameters that must be present in all client requests.
    pub mandatory_params: HashSet<String>,
    /// Upstream cincinnati service.
    pub path_prefix: String,
    /// Policy plugins.
    pub plugins: &'static [BoxedPlugin],
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            plugins: Box::leak(Box::new([])),
            mandatory_params: HashSet::new(),
            path_prefix: String::new(),
        }
    }
}
