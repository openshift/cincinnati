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
use config;
use daggy::{Dag, NodeIndex};
use failure::{Error, ResultExt};
use registry;
use semver::Version;
use serde_json;
use std::collections::HashMap;
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

pub fn run(opts: config::Options, state: State) -> ! {
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

#[derive(Debug, Serialize)]
pub struct Release {
    version: Version,
    payload: String,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct Empty {}

fn create_graph(opts: &config::Options) -> Result<Dag<Release, Empty>, Error> {
    fn create_node(
        p: &str,
        v: Version,
        m: HashMap<String, String>,
        d: &mut Dag<Release, Empty>,
        r: &mut HashMap<Version, NodeIndex>,
    ) -> Result<NodeIndex, Error> {
        // XXX Check if it exists
        //ensure!(
        //    dag.node_weight(*index).unwrap().defined == false,
        //    "Release {} defined multiple times", m.version
        //);
        Ok(*r.entry(v.clone()).or_insert_with(|| {
            d.add_node(Release {
                version: v,
                payload: p.to_string(),
                metadata: m,
            })
        }))
    }

    fn get_node(
        p: &str,
        v: Version,
        d: &mut Dag<Release, Empty>,
        r: &mut HashMap<Version, NodeIndex>,
    ) -> NodeIndex {
        *r.entry(v.clone()).or_insert_with(|| {
            d.add_node(Release {
                version: v,
                payload: p.to_string(),
                metadata: HashMap::new(),
            })
        })
    };

    let releases = registry::fetch_releases(&opts.registry, &opts.repository)
        .context("failed to fetch all release metadata")?;

    let mut dag = Dag::<Release, Empty>::new();
    let mut nodes = HashMap::<Version, NodeIndex>::new();

    for r in releases {
        let node = create_node(
            &r.source,
            r.metadata.version,
            r.metadata.metadata,
            &mut dag,
            &mut nodes,
        )?;

        for p in r.metadata.previous {
            let previous = get_node(&r.source, p, &mut dag, &mut nodes);
            dag.add_edge(previous, node, Empty {})?;
        }

        for n in r.metadata.next {
            let next = get_node(&r.source, n, &mut dag, &mut nodes);
            dag.add_edge(node, next, Empty {})?;
        }
    }

    Ok(dag)
}
