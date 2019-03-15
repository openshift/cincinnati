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

use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use cincinnati::{plugins, AbstractRelease, Graph, Release, CONTENT_TYPE};
use commons::GraphError;
use config;
use failure::Error;
use prometheus::{Counter, IntGauge};
use registry::{self, Registry};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;

lazy_static! {
    static ref GRAPH_FINAL_RELEASES: IntGauge = register_int_gauge!(
        "cincinnati_gb_graph_final_releases",
        "Number of releases in the final graph, after processing"
    )
    .unwrap();
    static ref GRAPH_UPSTREAM_RAW_RELEASES: IntGauge = register_int_gauge!(
        "cincinnati_gb_graph_upstream_raw_releases",
        "Number of releases fetched from upstream, before processing"
    )
    .unwrap();
    static ref UPSTREAM_SCRAPES: Counter = register_counter!(
        "cincinnati_gb_graph_upstream_scrapes_total",
        "Total number of upstream scrapes"
    )
    .unwrap();
    static ref UPSTREAM_ERRORS: Counter = register_counter!(
        "cincinnati_gb_graph_upstream_errors_total",
        "Total number of upstream scraping errors"
    )
    .unwrap();
    static ref V1_GRAPH_INCOMING_REQS: Counter = register_counter!(
        "cincinnati_gb_v1_graph_incoming_requests_total",
        "Total number of incoming HTTP client request to /v1/graph"
    )
    .unwrap();
}

pub fn index(req: HttpRequest<State>) -> Result<HttpResponse, GraphError> {
    V1_GRAPH_INCOMING_REQS.inc();

    // Check that the client can accept JSON media type.
    commons::ensure_content_type(req.headers(), CONTENT_TYPE)?;

    // Check for required client parameters.
    let mandatory_params = &req.state().mandatory_params;
    commons::ensure_query_params(mandatory_params, req.query_string())?;

    let resp = HttpResponse::Ok().content_type(CONTENT_TYPE).body(
        req.state()
            .json
            .read()
            .expect("json lock has been poisoned")
            .clone(),
    );
    Ok(resp)
}

#[derive(Clone)]
pub struct State {
    json: Arc<RwLock<String>>,
    /// Query parameters that must be present in all client requests.
    mandatory_params: HashSet<String>,
}

impl State {
    pub fn new(mandatory_params: HashSet<String>) -> State {
        State {
            json: Arc::new(RwLock::new(String::new())),
            mandatory_params,
        }
    }
}

pub fn run<'a>(opts: &'a config::Options, state: &State) -> ! {
    // Grow-only cache, mapping tag (hashed layers) to optional release metadata.
    let mut cache = HashMap::new();

    let registry = Registry::try_from_str(&opts.registry)
        .expect(&format!("failed to parse '{}' as Url", &opts.registry));

    // Read the credentials outside the loop to avoid re-reading the file
    let (username, password) =
        registry::read_credentials(opts.credentials_path.as_ref(), &registry.host)
            .expect("could not read registry credentials");

    let configured_plugins: Vec<Box<plugins::Plugin<plugins::PluginIO>>> =
        if opts.disable_quay_api_metadata {
            debug!("Disabling fetching and processing of quay metadata..");
            vec![]
        } else {
            use cincinnati::plugins::internal::{
                edge_add_remove::EdgeAddRemovePlugin, metadata_fetch_quay::QuayMetadataFetchPlugin,
                node_remove::NodeRemovePlugin,
            };
            use cincinnati::plugins::InternalPluginWrapper;

            // TODO(steveeJ): actually make this vec configurable
            vec![
                Box::new(InternalPluginWrapper(
                    QuayMetadataFetchPlugin::try_new(
                        opts.repository.clone(),
                        opts.quay_label_filter.clone(),
                        opts.quay_manifestref_key.clone(),
                        opts.quay_api_credentials_path.as_ref(),
                        opts.quay_api_base.clone(),
                    )
                    .expect("could not initialize the QuayMetadataPlugin"),
                )),
                Box::new(InternalPluginWrapper(NodeRemovePlugin {
                    key_prefix: opts.quay_label_filter.clone(),
                })),
                Box::new(InternalPluginWrapper(EdgeAddRemovePlugin {
                    key_prefix: opts.quay_label_filter.clone(),
                })),
            ]
        };

    // Don't wait on the first iteration
    let mut first_iteration = true;

    loop {
        if first_iteration {
            first_iteration = false;
        } else {
            thread::sleep(opts.period);
        }

        debug!("graph update triggered");

        let scrape = registry::fetch_releases(
            &registry,
            &opts.repository,
            username.as_ref().map(String::as_ref),
            password.as_ref().map(String::as_ref),
            &mut cache,
            &opts.quay_manifestref_key,
        );
        UPSTREAM_SCRAPES.inc();

        let releases = match scrape {
            Ok(releases) => {
                if releases.is_empty() {
                    warn!(
                        "could not find any releases in {}/{}",
                        &registry.host_port_string(),
                        &opts.repository
                    );
                };
                releases
            }
            Err(err) => {
                UPSTREAM_ERRORS.inc();
                err.iter_chain()
                    .for_each(|cause| error!("failed to fetch all release metadata: {}", cause));
                vec![]
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

        let graph = match cincinnati::plugins::process(
            &configured_plugins,
            cincinnati::plugins::InternalIO {
                graph,
                // the plugins used in the graph-builder don't expect any parameters yet
                parameters: Default::default(),
            },
        ) {
            Ok(graph) => graph,
            Err(err) => {
                err.iter_chain().for_each(|cause| error!("{}", cause));
                continue;
            }
        };

        match serde_json::to_string(&graph) {
            Ok(json) => {
                *state.json.write().expect("json lock has been poisoned") = json;
                let nodes_count = graph.releases_count();
                GRAPH_FINAL_RELEASES.set(nodes_count as i64);
                debug!("graph update completed, {} valid releases", nodes_count);
            }
            Err(err) => error!("Failed to serialize graph: {}", err),
        };
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
                graph.add_transition(&previous, &current)
            })?;

            next.iter().try_for_each(|version| {
                let next = match graph.find_by_version(&version.to_string()) {
                    Some(id) => id,
                    None => graph.add_release(Release::Abstract(AbstractRelease {
                        version: version.to_string(),
                    }))?,
                };
                graph.add_transition(&current, &next)
            })
        })?;

    Ok(graph)
}
