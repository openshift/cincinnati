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

use graph_builder::{config, graph, metrics};

use actix_web::{http::Method, middleware::Logger, server, App};
use failure::Error;
use std::{thread};

fn main() -> Result<(), Error> {
    let sys = actix::System::new("graph-builder");

    let opts = config::AppSettings::assemble()?;

    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), opts.verbosity)
        .init();
    debug!("application settings:\n{:#?}", &opts);

    let state = graph::State::new(opts.mandatory_client_parameters.clone());
    let service_addr = (opts.address, opts.port);
    let status_addr = (opts.status_address, opts.status_port);
    let app_prefix = opts.path_prefix.clone();

    {
        let state = state.clone();
        thread::spawn(move || graph::run(&opts, &state));
    }

    // Status service.
    server::new(|| {
        App::new()
            .middleware(Logger::default())
            .route("/metrics", Method::GET, metrics::serve)
    })
    .bind(status_addr)?
    .start();

    // Main service.
    server::new(move || {
        let app_prefix = app_prefix.clone();
        let state = state.clone();
        App::with_state(state)
            .middleware(Logger::default())
            .prefix(app_prefix)
            .route("/v1/graph", Method::GET, graph::index)
    })
    .bind(service_addr)?
    .start();

    sys.run();
    Ok(())
}
