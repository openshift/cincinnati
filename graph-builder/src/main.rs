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
extern crate tempfile;

use crate::failure::ResultExt;
use graph_builder::{config, graph, graph::RwLock, status};

use actix_web::{App, HttpServer};
use cincinnati::plugins::prelude::*;
use failure::Error;
use std::sync::Arc;
use std::thread;

fn main() -> Result<(), Error> {
    let sys = actix::System::new("graph-builder");

    let settings = config::AppSettings::assemble().context("could not assemble AppSettings")?;
    let plugins: Vec<BoxedPlugin> = if settings.disable_quay_api_metadata {
        Default::default()
    } else {
        // TODO(lucab): drop this when plugins are configurable.
        use cincinnati::plugins::internal::edge_add_remove::{
            EdgeAddRemovePlugin, DEFAULT_REMOVE_ALL_EDGES_VALUE,
        };
        use cincinnati::plugins::internal::metadata_fetch_quay::{
            QuayMetadataFetchPlugin, DEFAULT_QUAY_LABEL_FILTER, DEFAULT_QUAY_MANIFESTREF_KEY,
        };
        use cincinnati::plugins::internal::node_remove::NodeRemovePlugin;
        use quay::v1::DEFAULT_API_BASE;

        // TODO(steveeJ): actually make this vec configurable
        new_plugins!(
            InternalPluginWrapper(
                // TODO(lucab): source options from plugins config.
                QuayMetadataFetchPlugin::try_new(
                    settings.repository.clone(),
                    DEFAULT_QUAY_LABEL_FILTER.to_string(),
                    DEFAULT_QUAY_MANIFESTREF_KEY.to_string(),
                    None,
                    DEFAULT_API_BASE.to_string(),
                )
                .context("could not initialize the QuayMetadataPlugin")?,
            ),
            InternalPluginWrapper(NodeRemovePlugin {
                key_prefix: DEFAULT_QUAY_LABEL_FILTER.to_string(),
            }),
            InternalPluginWrapper(EdgeAddRemovePlugin {
                key_prefix: DEFAULT_QUAY_LABEL_FILTER.to_string(),
                remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),
            })
        )
    };

    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), settings.verbosity)
        .init();
    debug!("application settings:\n{:#?}", settings);

    let app_state = {
        let json_graph = Arc::new(RwLock::new(String::new()));
        let live = Arc::new(RwLock::new(false));
        let ready = Arc::new(RwLock::new(false));

        graph::State::new(
            json_graph.clone(),
            settings.mandatory_client_parameters.clone(),
            live.clone(),
            ready.clone(),
            Box::leak(Box::new(plugins)),
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
    HttpServer::new(move || {
        App::new()
            .register_data(actix_web::web::Data::new(status_state.clone()))
            .service(
                actix_web::web::resource("/liveness")
                    .route(actix_web::web::get().to(status::serve_liveness)),
            )
            .service(
                actix_web::web::resource("/metrics")
                    .route(actix_web::web::get().to(status::serve_metrics)),
            )
            .service(
                actix_web::web::resource("/readiness")
                    .route(actix_web::web::get().to(status::serve_readiness)),
            )
    })
    .bind(status_addr)?
    .start();

    // Main service.
    let main_state = app_state.clone();
    HttpServer::new(move || {
        App::new()
            .register_data(actix_web::web::Data::new(main_state.clone()))
            .service(
                actix_web::web::resource(&format!("{}/v1/graph", app_prefix.clone()))
                    .route(actix_web::web::get().to(graph::index)),
            )
    })
    .bind(service_addr)?
    .start();

    let _ = sys.run();

    Ok(())
}
