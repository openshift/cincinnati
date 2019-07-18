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

use crate::config;
use crate::registry::{self, Registry};
use actix_web::{HttpRequest, HttpResponse};
use cincinnati::plugins::prelude::*;
use cincinnati::{AbstractRelease, Graph, Release, CONTENT_TYPE};
use commons::GraphError;
use failure::{Error, Fallible};
use futures::Future;
use lazy_static;
pub use parking_lot::RwLock;
use prometheus::{self, Counter, IntGauge};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;

lazy_static! {
    static ref GRAPH_FINAL_RELEASES: IntGauge = IntGauge::new(
        "graph_final_releases",
        "Number of releases in the final graph, after processing"
    )
    .unwrap();
    static ref GRAPH_LAST_SUCCESSFUL_REFRESH: IntGauge = IntGauge::new(
        "graph_last_successful_refresh_timestamp",
        "UTC timestamp of last successful graph refresh"
    )
    .unwrap();
    static ref GRAPH_UPSTREAM_RAW_RELEASES: IntGauge = IntGauge::new(
        "graph_upstream_raw_releases",
        "Number of releases fetched from upstream, before processing"
    )
    .unwrap();
    static ref UPSTREAM_ERRORS: Counter = Counter::new(
        "graph_upstream_errors_total",
        "Total number of upstream scraping errors"
    )
    .unwrap();
    static ref UPSTREAM_SCRAPES: Counter = Counter::new(
        "graph_upstream_scrapes_total",
        "Total number of upstream scrapes"
    )
    .unwrap();
    static ref V1_GRAPH_INCOMING_REQS: Counter = Counter::new(
        "v1_graph_incoming_requests_total",
        "Total number of incoming HTTP client request to /v1/graph"
    )
    .unwrap();
}

/// Register relevant metrics to a prometheus registry.
pub fn register_metrics(registry: &prometheus::Registry) -> Fallible<()> {
    commons::register_metrics(&registry)?;
    registry.register(Box::new(GRAPH_FINAL_RELEASES.clone()))?;
    registry.register(Box::new(GRAPH_LAST_SUCCESSFUL_REFRESH.clone()))?;
    registry.register(Box::new(GRAPH_UPSTREAM_RAW_RELEASES.clone()))?;
    registry.register(Box::new(UPSTREAM_ERRORS.clone()))?;
    registry.register(Box::new(UPSTREAM_SCRAPES.clone()))?;
    registry.register(Box::new(V1_GRAPH_INCOMING_REQS.clone()))?;
    Ok(())
}

/// Serve Cincinnati graph requests.
pub fn index(req: HttpRequest) -> Result<HttpResponse, GraphError> {
    V1_GRAPH_INCOMING_REQS.inc();

    // Check that the client can accept JSON media type.
    commons::ensure_content_type(req.headers(), CONTENT_TYPE)?;

    // Check for required client parameters.
    let mandatory_params = &req
        .app_data::<State>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .mandatory_params;
    commons::ensure_query_params(mandatory_params, req.query_string())?;

    let resp = HttpResponse::Ok().content_type(CONTENT_TYPE).body(
        req.app_data::<State>()
            .expect(commons::MISSING_APPSTATE_PANIC_MSG)
            .json
            .read()
            .clone(),
    );
    Ok(resp)
}

#[derive(Clone)]
pub struct State {
    json: Arc<RwLock<String>>,
    /// Query parameters that must be present in all client requests.
    mandatory_params: HashSet<String>,
    live: Arc<RwLock<bool>>,
    ready: Arc<RwLock<bool>>,
    plugins: &'static [BoxedPlugin],
}

impl State {
    /// Creates a new State with the given arguments
    pub fn new(
        json: Arc<RwLock<String>>,
        mandatory_params: HashSet<String>,
        live: Arc<RwLock<bool>>,
        ready: Arc<RwLock<bool>>,
        plugins: &'static [BoxedPlugin],
    ) -> State {
        State {
            json,
            mandatory_params,
            live,
            ready,
            plugins,
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

#[allow(clippy::useless_let_if_seq)]
pub fn run<'a>(settings: &'a config::AppSettings, state: &State) -> ! {
    // Grow-only cache, mapping tag (hashed layers) to optional release metadata.
    let mut cache = HashMap::new();

    let registry = Registry::try_from_str(&settings.registry)
        .unwrap_or_else(|_| panic!("failed to parse '{}' as Url", &settings.registry));

    // Read the credentials outside the loop to avoid re-reading the file
    let (username, password) =
        registry::read_credentials(settings.credentials_path.as_ref(), &registry.host)
            .expect("could not read registry credentials");

    // Indicate if a panic happens
    let previous_hook = std::panic::take_hook();
    let panic_live = state.live.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        *panic_live.write() = false;
        previous_hook(panic_info)
    }));

    // Don't wait on the first iteration
    let mut first_iteration = true;
    let mut first_success = true;

    loop {
        if first_iteration {
            *state.live.write() = true;
            first_iteration = false;
        } else {
            thread::sleep(settings.pause_secs);
        }

        debug!("graph update triggered");

        let scrape = registry::fetch_releases(
            &registry,
            &settings.repository,
            username.as_ref().map(String::as_ref),
            password.as_ref().map(String::as_ref),
            &mut cache,
            &settings.manifestref_key,
        );
        UPSTREAM_SCRAPES.inc();

        let releases = match scrape {
            Ok(releases) => {
                if releases.is_empty() {
                    warn!(
                        "could not find any releases in {}/{}",
                        &registry.host_port_string(),
                        &settings.repository
                    );
                };
                releases
            }
            Err(err) => {
                UPSTREAM_ERRORS.inc();
                err.iter_chain()
                    .for_each(|cause| error!("failed to fetch all release metadata: {}", cause));
                continue;
            }
        };
        GRAPH_UPSTREAM_RAW_RELEASES.set(releases.len() as i64);

        let graph = match create_graph(releases) {
            Ok(graph) => graph,
            Err(err) => {
                err.iter_chain().for_each(|cause| error!("{}", cause));
                continue;
            }
        };

        let future_graph = cincinnati::plugins::process(
            state.plugins.iter(),
            cincinnati::plugins::PluginIO::InternalIO(cincinnati::plugins::InternalIO {
                graph,
                // the plugins used in the graph-builder don't expect any parameters yet
                parameters: Default::default(),
            }),
        )
        .map(|internal_io| internal_io.graph);

        let graph = match tokio::runtime::current_thread::Runtime::new()
            .unwrap()
            .block_on(future_graph)
        {
            Ok(graph) => graph,
            Err(err) => {
                err.iter_chain().for_each(|cause| error!("{}", cause));
                continue;
            }
        };

        let json_graph = match serde_json::to_string(&graph) {
            Ok(json) => json,
            Err(err) => {
                error!("Failed to serialize graph: {}", err);
                continue;
            }
        };

        *state.json.write() = json_graph;

        if first_success {
            *state.ready.write() = true;
            first_success = false;
        };

        GRAPH_LAST_SUCCESSFUL_REFRESH.set(chrono::Utc::now().timestamp() as i64);

        let nodes_count = graph.releases_count();
        GRAPH_FINAL_RELEASES.set(nodes_count as i64);
        debug!("graph update completed, {} valid releases", nodes_count);
    }
}

pub fn create_graph(releases: Vec<registry::Release>) -> Result<Graph, Error> {
    let mut graph = Graph::default();

    releases
        .into_iter()
        .inspect(|release| trace!("Adding a release to the graph '{:?}'", release))
        .try_for_each(|release| {
            let previous = release.metadata.previous.clone();
            let next = release.metadata.next.clone();
            let current = graph.add_release(release)?;

            previous.iter().try_for_each(|version| {
                let previous = match graph.find_by_version(&version.to_string()) {
                    Some(id) => id,
                    None => graph.add_release(Release::Abstract(AbstractRelease {
                        version: version.to_string(),
                    }))?,
                };
                graph.add_edge(&previous, &current)
            })?;

            next.iter().try_for_each(|version| {
                let next = match graph.find_by_version(&version.to_string()) {
                    Some(id) => id,
                    None => graph.add_release(Release::Abstract(AbstractRelease {
                        version: version.to_string(),
                    }))?,
                };
                graph.add_edge(&current, &next)
            })
        })?;

    Ok(graph)
}
