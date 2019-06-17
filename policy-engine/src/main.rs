//! Cincinnati backend: policy-engine server.

#![deny(missing_docs)]

extern crate actix;
extern crate actix_web;
extern crate cincinnati;
#[macro_use]
extern crate commons;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate prometheus;
extern crate semver;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate smart_default;
#[macro_use]
extern crate structopt;
extern crate openapiv3;
extern crate url;

mod config;
mod graph;
mod metrics;
mod openapi;

use actix_web::{App, HttpServer};
use failure::Error;
use std::collections::HashSet;

fn main() -> Result<(), Error> {
    let sys = actix::System::new("policy-engine");

    let settings = config::AppSettings::assemble()?;

    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), settings.verbosity)
        .init();
    debug!("application settings:\n{:#?}", &settings);

    // Metrics service.
    graph::register_metrics(&metrics::PROM_REGISTRY)?;
    HttpServer::new(|| {
        App::new().service(
            actix_web::web::resource("/metrics").route(actix_web::web::get().to(metrics::serve)),
        )
    })
    .bind((settings.status_address, settings.status_port))?
    .start();

    // Main service.
    let state = AppState {
        mandatory_params: settings.mandatory_client_parameters.clone(),
        upstream: settings.upstream.clone(),
        path_prefix: settings.path_prefix.clone(),
    };

    HttpServer::new(move || {
        let app_prefix = state.path_prefix.clone();
        App::new()
            .register_data(actix_web::web::Data::new(state.clone()))
            .service(
                actix_web::web::resource(&format!("{}/v1/graph", app_prefix))
                    .route(actix_web::web::get().to(graph::index)),
            )
            .service(
                actix_web::web::resource(&format!("{}/v1/openapi", app_prefix))
                    .route(actix_web::web::get().to(openapi::index)),
            )
    })
    .bind((settings.address, settings.port))?
    .start();

    let _ = sys.run();
    Ok(())
}

/// Shared application configuration (cloned per-thread).
#[derive(Debug, Clone)]
struct AppState {
    /// Query parameters that must be present in all client requests.
    pub mandatory_params: HashSet<String>,
    /// Upstream cincinnati service.
    pub upstream: hyper::Uri,
    /// Common namespace for API endpoints.
    pub path_prefix: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mandatory_params: HashSet::new(),
            upstream: hyper::Uri::from_static(config::DEFAULT_UPSTREAM_URL),
            path_prefix: String::new(),
        }
    }
}
