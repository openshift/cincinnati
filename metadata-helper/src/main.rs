// Copyright 2023 Pratik Mahajan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
mod status;
mod signatures;

use actix_cors::Cors;
use actix_service::Service;
use actix_web::http::StatusCode;
use actix_web::{middleware, App, HttpRequest, HttpResponse, HttpServer};
use commons::metrics::{self, HasRegistry};
use commons::prelude_errors::*;
use commons::tracing::{get_tracer, init_tracer, set_span_tags};
use futures::future;
use opentelemetry::{
    trace::{mark_span_as_active, FutureExt, Tracer},
    Context as ot_context,
};
use parking_lot::RwLock;
use prometheus::{labels, opts, Counter, Registry};
use std::sync::Arc;

/// Common prefix for metadata-helper metrics.
pub static METRICS_PREFIX: &str = "metadata-helper";


#[allow(dead_code)]
/// Build info
mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

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

    // Shared state.
    let state = {
        let path_prefix = settings.path_prefix.clone();
        let live = Arc::new(RwLock::new(false));
        let ready = Arc::new(RwLock::new(false));

        AppState::new(
            path_prefix,
            live,
            ready,
            registry,
        )
    };

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
    init_tracer(METRICS_PREFIX, settings.tracing_endpoint.clone())?;
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
                actix_web::web::resource(&format!("{}/signatures", app_prefix))
                    .route(actix_web::web::get().to(status::serve_readiness)),
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
    *state.ready.write() = true;

    BUILD_INFO.inc();
    future::try_join(metrics_server, main_server).await?;
    Ok(())
}


// log errors in case an incorrect endpoint is called
async fn default_response(req: HttpRequest) -> HttpResponse {
    error!(
        "Error serving request from '{}': Incorrect Endpoint",
        req.peer_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "<not available>".into())
    );
    HttpResponse::new(StatusCode::NOT_FOUND)
}


/// Shared application configuration (cloned per-thread).
#[derive(Clone, Debug)]
pub struct AppState {
    /// Upstream cincinnati service.
    path_prefix: String,
    live: Arc<RwLock<bool>>,
    ready: Arc<RwLock<bool>>,
    registry: &'static Registry,
}


impl AppState {
    /// Creates a new State with the given arguments
    pub fn new(
        path_prefix: String,
        live: Arc<RwLock<bool>>,
        ready: Arc<RwLock<bool>>,
        registry: &'static Registry,
    ) -> AppState {
        AppState {
            path_prefix,
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

impl HasRegistry for AppState {
    fn registry(&self) -> &'static Registry {
        self.registry
    }
}
