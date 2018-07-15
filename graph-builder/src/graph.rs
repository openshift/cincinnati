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

use actix_web::{HttpRequest, HttpResponse};
use cincinnati::{AbstractRelease, Graph, Release};
use config;
use failure::{Error, ResultExt};
use registry;
use serde_json;
use std::sync::{Arc, RwLock};
use std::thread;

pub fn index(req: HttpRequest<State>) -> HttpResponse {
    HttpResponse::Ok().content_type("application/json").body(
        req.state()
            .json
            .read()
            .expect("json lock has been poisoned")
            .clone(),
    )
}

#[derive(Clone)]
pub struct State {
    json: Arc<RwLock<String>>,
}

impl State {
    pub fn new() -> State {
        State {
            json: Arc::new(RwLock::new(String::new())),
        }
    }
}

pub fn run(opts: &config::Options, state: &State) -> ! {
    loop {
        debug!("Updating graph...");
        match create_graph(&opts) {
            Ok(graph) => match serde_json::to_string(&graph) {
                Ok(json) => *state.json.write().expect("json lock has been poisoned") = json,
                Err(err) => error!("Failed to serialize graph: {}", err),
            },
            Err(err) => err.causes().for_each(|cause| error!("{}", cause)),
        }
        thread::sleep(opts.period);
    }
}

fn create_graph(opts: &config::Options) -> Result<Graph, Error> {
    let mut graph = Graph::default();

    registry::fetch_releases(&opts.registry, &opts.repository)
        .context("failed to fetch all release metadata")?
        .into_iter()
        .try_for_each(|release| {
            let previous = release.metadata.previous.clone();
            let next = release.metadata.next.clone();
            let current = graph.add_release(release)?;

            previous.iter().try_for_each(|version| {
                let previous = match graph.find_by_version(version) {
                    Some(id) => id,
                    None => graph.add_release(Release::Abstract(AbstractRelease {
                        version: version.clone(),
                    }))?,
                };
                graph.add_transition(&previous, &current)
            })?;

            next.iter().try_for_each(|version| {
                let next = match graph.find_by_version(version) {
                    Some(id) => id,
                    None => graph.add_release(Release::Abstract(AbstractRelease {
                        version: version.clone(),
                    }))?,
                };
                graph.add_transition(&current, &next)
            })
        })?;

    Ok(graph)
}
