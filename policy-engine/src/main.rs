//! Cincinnati backend: policy-engine server.

#![deny(missing_docs)]

extern crate actix;
extern crate actix_web;
#[macro_use]
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
#[macro_use]
extern crate custom_debug_derive;

extern crate tempfile;
extern crate url;

mod config;
mod graph;
mod openapi;

use actix_web::{App, HttpServer};
use cincinnati::plugins::BoxedPlugin;
use commons::metrics::{self, RegistryWrapper};
use failure::Error;
use prometheus::Registry;
use std::collections::HashSet;

/// Common prefix for policy-engine metrics.
pub static METRICS_PREFIX: &str = "cincinnati_pe";

fn main() -> Result<(), Error> {
    let sys = actix::System::new("policy-engine");

    let settings = config::AppSettings::assemble()?;
    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), settings.verbosity)
        .init();
    debug!("application settings:\n{:#?}", &settings);

    // Metrics service.
    let registry: &'static Registry = Box::leak(Box::new(metrics::new_registry(Some(
        METRICS_PREFIX.to_string(),
    ))?));
    graph::register_metrics(registry)?;
    HttpServer::new(move || {
        App::new()
            .register_data(actix_web::web::Data::new(RegistryWrapper(registry)))
            .service(
                actix_web::web::resource("/metrics")
                    .route(actix_web::web::get().to(metrics::serve::<RegistryWrapper>)),
            )
    })
    .bind((settings.status_address, settings.status_port))?
    .start();

    // Main service.
    let plugins = settings.policy_plugins(Some(registry))?;
    let state = AppState {
        mandatory_params: settings.mandatory_client_parameters.clone(),
        path_prefix: settings.path_prefix.clone(),
        plugins: Box::leak(Box::new(plugins)),
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
