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
use cincinnati::{AbstractRelease, Graph, Release, CONTENT_TYPE};
use commons::GraphError;
use config;
use failure::Error;
use metadata::{fetch_and_populate_dynamic_metadata, MetadataFetcher, QuayMetadataFetcher};
use registry::{self, Registry};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;

pub fn index(req: HttpRequest<State>) -> Result<HttpResponse, GraphError> {
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

    let metadata_fetcher: Option<MetadataFetcher> = if opts.disable_quay_api_metadata {
        debug!("Disable fetching of quay metadata..");
        None
    } else {
        Some(
            QuayMetadataFetcher::try_new(
                opts.quay_label_filter.clone(),
                opts.quay_api_credentials_path.as_ref(),
                opts.quay_api_base.clone(),
                opts.repository.clone(),
            )
            .expect("try_new to yield a metadata fetcher"),
        )
    };

    let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();

    // Don't wait on the first iteration
    let mut first_iteration = true;

    loop {
        if first_iteration {
            first_iteration = false;
        } else {
            thread::sleep(opts.period);
        }

        debug!("graph update triggered");

        let mut releases = match registry::fetch_releases(
            &registry,
            &opts.repository,
            username.as_ref().map(String::as_ref),
            password.as_ref().map(String::as_ref),
            &mut cache,
        ) {
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
                err.iter_chain()
                    .for_each(|cause| error!("failed to fetch all release metadata: {}", cause));
                vec![]
            }
        };

        if let Some(metadata_fetcher) = &metadata_fetcher {
            match runtime.block_on(fetch_and_populate_dynamic_metadata(
                metadata_fetcher,
                releases.clone(),
            )) {
                Ok(populated_releases) => {
                    releases = populated_releases;
                }
                Err(err) => {
                    err.iter_chain()
                        .for_each(|cause| error!("failed to fetch dynamic metadata: {}", cause));
                    continue;
                }
            }
        };

        let graph = match create_graph(releases, &opts.quay_label_filter) {
            Ok(graph) => Some(graph),
            Err(err) => {
                err.iter_chain().for_each(|cause| error!("{}", cause));
                continue;
            }
        };

        if let Some(graph) = graph {
            match serde_json::to_string(&graph) {
                Ok(json) => {
                    *state.json.write().expect("json lock has been poisoned") = json;
                    debug!(
                        "graph update completed, {} valid releases",
                        graph.releases_count()
                    );
                }
                Err(err) => error!("Failed to serialize graph: {}", err),
            }
        };
    }
}

pub fn create_graph(
    releases: Vec<registry::Release>,
    quay_label_filter: &str,
) -> Result<Graph, Error> {
    let mut graph = Graph::default();

    releases
        .into_iter()
        .filter_map(|release| process_release_remove_label(quay_label_filter, release))
        .map(|release| process_neighbor_labels(quay_label_filter, release))
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

    graph.prune_abstract();

    Ok(graph)
}

/// Removes and retrieves the metadata value of the given `key` which is prefixed
/// by `filter`
fn remove_prefixed_label(
    release: &mut registry::Release,
    filter: &str,
    key: &str,
) -> Option<String> {
    release
        .metadata
        .metadata
        .remove(&format!("{}.{}", filter, key))
}

/// Remove releases which are labeled for removal.
///
/// The labels are assumed to have the syntax `{filter}.release-remove=<bool>`
fn process_release_remove_label(
    filter: &str,
    release: registry::Release,
) -> Option<registry::Release> {
    let mut release = release;

    if let Some(s) = remove_prefixed_label(&mut release, filter, "release.remove") {
        // Filter releases which have "{prefix}.release-remove=true"
        let remove = std::str::FromStr::from_str(&s).unwrap_or_else(|e| {
            error!("could not parse '{}' as bool: {} (assuming false)", s, e);
            false
        });

        if remove {
            debug!("Removing release '{:?}'", release);
            return None;
        }
    };

    Some(release)
}

/// Add next and previous releases from quay labels
///
/// The labels are assumed to have the syntax `{prefix}.(previous|next)=[<Version>, ...]>`
fn process_neighbor_labels(filter: &str, release: registry::Release) -> registry::Release {
    use semver::Version;
    use std::collections::HashSet;

    let mut release = release;

    macro_rules! process_neighbor_add_labels {
        ($dir:tt, $neighbors:expr) => {
            if let Some(s) = remove_prefixed_label(&mut release, filter, $dir) {
                let mut neighbors_additional: Vec<Version> = s
                    .split(',')
                    .map(|neighbor_version| -> Version {
                        let version = Version::parse(neighbor_version).unwrap();
                        debug!(
                            "Adding neighbor '{}' to '{}' due to label '{}={}' ",
                            version, release.metadata.version, $dir, neighbor_version
                        );
                        version
                    })
                    .collect();

                $neighbors.append(&mut neighbors_additional);
            };
        };
    }
    process_neighbor_add_labels!("previous.add", &mut release.metadata.previous);
    process_neighbor_add_labels!("next.add", &mut release.metadata.next);

    macro_rules! process_neighbor_remove_labels {
        ($dir:tt, $neighbors:expr) => {
            if let Some(s) = remove_prefixed_label(&mut release, filter, $dir) {
                let mut neighbors_to_remove: HashSet<Version> = s
                    .split(',')
                    .map(|neighbor_version| -> Version {
                        let version = Version::parse(neighbor_version).unwrap();
                        debug!(
                            "Removing neighbor '{}' from '{}' due to label '{}={}' ",
                            version, release.metadata.version, $dir, neighbor_version
                        );
                        version
                    })
                    .collect();

                $neighbors.retain(|version| !neighbors_to_remove.contains(&version));
            };
        };
    }
    process_neighbor_remove_labels!("previous.remove", &mut release.metadata.previous);
    process_neighbor_remove_labels!("next.remove", &mut release.metadata.next);

    release
}
