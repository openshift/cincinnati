//! Cincinnati backend: policy-engine server.

extern crate actix;
extern crate actix_web;
extern crate cincinnati;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
extern crate log;
#[macro_use]
extern crate prometheus;
extern crate semver;
extern crate serde_json;
#[macro_use]
extern crate structopt;

mod config;
mod graph;
mod metrics;

use actix_web::{http::Method, middleware::Logger, server, App};
use failure::Error;
use log::LevelFilter;
use structopt::StructOpt;

fn main() -> Result<(), Error> {
    let sys = actix::System::new("policy-engine");
    let opts = config::Options::from_args();

    env_logger::Builder::from_default_env()
        .filter(
            Some(module_path!()),
            match opts.verbosity {
                0 => LevelFilter::Warn,
                1 => LevelFilter::Info,
                2 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            },
        )
        .init();

    // Metrics service.
    server::new(move || {
        App::new()
            .middleware(Logger::default())
            .route("/metrics", Method::GET, metrics::serve)
    })
    .bind((opts.metrics_address, opts.metrics_port))?
    .start();

    // Main service.
    let state = graph::State {
        upstream: opts.upstream,
    };
    server::new(move || {
        App::with_state(state.clone())
            .middleware(Logger::default())
            .route("/v1/graph", Method::GET, graph::index)
    })
    .bind((opts.address, opts.port))?
    .start();

    sys.run();
    Ok(())
}
