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

extern crate actix_web;
extern crate cincinnati;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate hyper;
extern crate log;
extern crate semver;
extern crate serde_json;
#[macro_use]
extern crate structopt;

mod config;
mod graph;

use actix_web::{http::Method, middleware::Logger, server, App};
use failure::Error;
use log::LevelFilter;
use structopt::StructOpt;

fn main() -> Result<(), Error> {
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

    let state = graph::State {
        upstream: opts.upstream,
    };
    server::new(move || {
        App::with_state(state.clone())
            .middleware(Logger::default())
            .route("/v1/graph", Method::GET, graph::index)
    }).bind((opts.address, opts.port))?
        .run();
    Ok(())
}
