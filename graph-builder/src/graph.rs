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

use actix_web::http::header::{self, HeaderValue};
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use cincinnati::{AbstractRelease, Graph, Release, CONTENT_TYPE};
use config;
use failure::Error;
use registry::{self, Registry};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;

pub fn index(req: HttpRequest<State>) -> HttpResponse {
    // Check for required client parameters.
    let mandatory_params = &req.state().mandatory_params;
    if commons::ensure_query_params(mandatory_params, req.query_string()).is_err() {
        return HttpResponse::BadRequest().finish();
    }

    match req.headers().get(header::ACCEPT) {
        Some(entry) if entry == HeaderValue::from_static(CONTENT_TYPE) => {
            HttpResponse::Ok().content_type(CONTENT_TYPE).body(
                req.state()
                    .json
                    .read()
                    .expect("json lock has been poisoned")
                    .clone(),
            )
        }
        _ => HttpResponse::NotAcceptable().finish(),
    }
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

pub fn run(opts: &config::Options, state: &State) -> ! {
    // Grow-only cache, mapping tag (hashed layers) to optional release metadata.
    let mut cache = HashMap::new();

    let registry = Registry::try_from_str(&opts.registry)
        .expect(&format!("failed to parse '{}' as Url", &opts.registry));

    // Read the credentials outside the loop to avoid re-reading the file
    let (username, password) =
        registry::read_credentials(opts.credentials_path.as_ref(), &registry.host)
            .expect("could not read registry credentials");

    let quay_api_token = quay::read_credentials(opts.quay_api_credentials_path.as_ref())
        .expect("could not read quay API credentials");

    loop {
        debug!("graph update triggered");

        let releases = match registry::fetch_releases(
            &registry,
            &opts.repository,
            &opts.quay_label_filter,
            quay_api_token.as_ref().map(String::as_str),
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

        match create_graph(releases) {
            Ok(graph) => match serde_json::to_string(&graph) {
                Ok(json) => {
                    *state.json.write().expect("json lock has been poisoned") = json;
                    debug!(
                        "graph update completed, {} valid releases",
                        graph.releases_count()
                    );
                }
                Err(err) => error!("Failed to serialize graph: {}", err),
            },
            Err(err) => err.iter_chain().for_each(|cause| error!("{}", cause)),
        }
        thread::sleep(opts.period);
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
