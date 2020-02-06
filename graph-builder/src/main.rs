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
use actix_web::{App, HttpServer};
use cincinnati::plugins::prelude::*;
use commons::metrics::{self, HasRegistry};
use failure::Error;
use graph_builder::{config, graph, graph::RwLock, status};
use std::sync::Arc;
use std::thread;

/// Common prefix for graph-builder metrics.
pub static METRICS_PREFIX: &str = "cincinnati_gb";

fn main() -> Result<(), Error> {
    let sys = actix::System::new("graph-builder");

    let settings = config::AppSettings::assemble().context("could not assemble AppSettings")?;
    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), settings.verbosity)
        .init();
    debug!("application settings:\n{:#?}", settings);

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
    let registry: prometheus::Registry = metrics::new_registry(Some(METRICS_PREFIX.to_string()))?;

    let service_addr = (settings.address, settings.port);
    let status_addr = (settings.status_address, settings.status_port);
    let app_prefix = settings.path_prefix.clone();

    // Shared state.
    let state = {
        let json_graph = Arc::new(RwLock::new(String::new()));
        let live = Arc::new(RwLock::new(false));
        let ready = Arc::new(RwLock::new(false));

        graph::State::new(
            json_graph,
            settings.mandatory_client_parameters.clone(),
            live,
            ready,
            Box::leak(Box::new(plugins)),
            Box::leak(Box::new(registry)),
        )
    };

    // Graph scraper
    {
        let mut runtime = tokio::runtime::Runtime::new().unwrap();

        let graph_state = state.clone();
        thread::spawn(move || runtime.block_on(graph::run(&settings, &graph_state)));
    }

    // Status service.
    graph::register_metrics(state.registry())?;

    let status_state = state.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(status_state.clone()))
            .service(
                actix_web::web::resource("/liveness")
                    .route(actix_web::web::get().to(status::serve_liveness)),
            )
            .service(
                actix_web::web::resource("/metrics")
                    .route(actix_web::web::get().to(metrics::serve::<graph::State>)),
            )
            .service(
                actix_web::web::resource("/readiness")
                    .route(actix_web::web::get().to(status::serve_readiness)),
            )
    })
    .bind(status_addr)?
    .run();

    // Main service.
    let main_state = state;
    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(main_state.clone()))
            .service(
                actix_web::web::resource(&format!("{}/v1/graph", app_prefix.clone()))
                    .route(actix_web::web::get().to(graph::index)),
            )
    })
    .bind(service_addr)?
    .run();

    let _ = sys.run();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::State;
    use commons::metrics::HasRegistry;
    use commons::metrics::RegistryWrapper;
    use commons::testing;
    use failure::{bail, Fallible};
    use parking_lot::RwLock;
    use prometheus::Registry;
    use std::collections::HashSet;
    use std::sync::Arc;

    fn mock_state() -> State {
        let json_graph = Arc::new(RwLock::new(String::new()));
        let live = Arc::new(RwLock::new(false));
        let ready = Arc::new(RwLock::new(false));

        let plugins = Box::leak(Box::new([]));
        let registry: &'static Registry = Box::leak(Box::new(
            metrics::new_registry(Some(METRICS_PREFIX.to_string())).unwrap(),
        ));

        State::new(json_graph, HashSet::new(), live, ready, plugins, registry)
    }

    #[test]
    fn serve_metrics_basic() -> Fallible<()> {
        let mut rt = testing::init_runtime()?;
        let state = mock_state();

        let registry = <dyn HasRegistry>::registry(&state);
        graph::register_metrics(registry)?;
        testing::dummy_gauge(registry, 42.0)?;

        let metrics_call =
            metrics::serve::<RegistryWrapper>(actix_web::web::Data::new(RegistryWrapper(registry)));
        let resp = rt.block_on(metrics_call)?;

        assert_eq!(resp.status(), 200);
        if let actix_web::body::ResponseBody::Body(body) = resp.body() {
            if let actix_web::body::Body::Bytes(bytes) = body {
                assert!(!bytes.is_empty());
                println!("{:?}", std::str::from_utf8(bytes.as_ref()));
                assert!(
                    twoway::find_bytes(bytes.as_ref(), b"cincinnati_gb_dummy_gauge 42\n").is_some()
                );
            } else {
                bail!("expected Body")
            }
        } else {
            bail!("expected bytes in body")
        };

        Ok(())
    }
}
