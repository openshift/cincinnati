// Copyright 2018 Alex Crawford
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

extern crate actix;
extern crate actix_web;
extern crate failure;
extern crate graph_builder;
#[macro_use]
extern crate log;
extern crate structopt;

use graph_builder::{config, graph, graph::RwLock, status};

use actix_web::{http::Method, middleware::Logger, server, App};
use failure::Error;
use std::sync::Arc;
use std::thread;

fn main() -> Result<(), Error> {
    let sys = actix::System::new("graph-builder");

    let settings = config::AppSettings::assemble()?;

    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), settings.verbosity)
        .init();
    debug!("application settings:\n{:#?}", &settings);

    let app_state = {
        let json_graph = Arc::new(RwLock::new(String::new()));
        let live = Arc::new(RwLock::new(false));
        let ready = Arc::new(RwLock::new(false));

        graph::State::new(
            json_graph.clone(),
            settings.mandatory_client_parameters.clone(),
            live.clone(),
            ready.clone(),
        )
    };

    let service_addr = (settings.address, settings.port);
    let status_addr = (settings.status_address, settings.status_port);
    let app_prefix = settings.path_prefix.clone();

    // Graph scraper
    let graph_state = app_state.clone();
    thread::spawn(move || graph::run(&settings, &graph_state));

    // Status service.
    graph::register_metrics(&status::PROM_REGISTRY)?;
    let status_state = app_state.clone();
    server::new(move || {
        App::with_state(status_state.clone())
            .middleware(Logger::default())
            .route("/liveness", Method::GET, status::serve_liveness)
            .route("/metrics", Method::GET, status::serve_metrics)
            .route("/readiness", Method::GET, status::serve_readiness)
    })
    .bind(status_addr)?
    .start();

    // Main service.
    let main_state = app_state.clone();
    server::new(move || {
        App::with_state(main_state.clone())
            .middleware(Logger::default())
            .prefix(app_prefix.clone())
            .route("/v1/graph", Method::GET, graph::index)
    })
    .bind(service_addr)?
    .start();

    sys.run();
    Ok(())
}
