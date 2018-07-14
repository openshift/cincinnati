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

extern crate daggy;
#[macro_use]
extern crate failure;
extern crate semver;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use daggy::petgraph::visit::{IntoNodeReferences, NodeRef};
use daggy::{Dag, Walker};
use failure::Error;
use semver::Version;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::collections::HashMap;

#[derive(Default)]
pub struct Graph {
    dag: Dag<Release, Empty>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Release {
    Abstract(AbstractRelease),
    Concrete(ConcreteRelease),
}

impl Release {
    pub fn version(&self) -> &Version {
        match self {
            Release::Abstract(release) => &release.version,
            Release::Concrete(release) => &release.version,
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub struct AbstractRelease {
    pub version: Version,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ConcreteRelease {
    pub version: Version,
    pub payload: String,
    pub metadata: HashMap<String, String>,
}

pub struct ReleaseId(daggy::petgraph::graph::NodeIndex);

pub struct NextReleases<'a> {
    children: daggy::Children<Release, Empty, daggy::petgraph::graph::DefaultIx>,
    dag: &'a Dag<Release, Empty>,
}

impl<'a> Iterator for NextReleases<'a> {
    type Item = &'a Release;

    fn next(&mut self) -> Option<Self::Item> {
        self.children
            .walk_next(self.dag)
            .map(|(_, i)| self.dag.node_weight(i).unwrap())
    }
}

struct Empty {}

impl Graph {
    pub fn add_release<R>(&mut self, release: R) -> Result<ReleaseId, Error>
    where
        R: Into<Release>,
    {
        let release = release.into();
        match self.find_by_version(&release.version()) {
            Some(id) => {
                let mut node = self.dag.node_weight_mut(id.0).unwrap();
                if let Release::Concrete(_) = node {
                    bail!(
                        "Concrete release with the same version ({}) already exists",
                        release.version()
                    );
                }
                *node = release;
                Ok(id)
            }
            None => Ok(ReleaseId(self.dag.add_node(release))),
        }
    }

    pub fn add_transition(&mut self, source: &ReleaseId, target: &ReleaseId) -> Result<(), Error> {
        self.dag.add_edge(source.0, target.0, Empty {})?;
        Ok(())
    }

    pub fn find_by_version(&self, version: &Version) -> Option<ReleaseId> {
        self.dag
            .node_references()
            .find(|nr| nr.weight().version() == version)
            .map(|nr| ReleaseId(nr.id()))
    }

    pub fn next_releases(&self, source: &ReleaseId) -> NextReleases {
        NextReleases {
            children: self.dag.children(source.0),
            dag: &self.dag,
        }
    }
}

impl Serialize for Graph {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        struct Edges<'a>(&'a [daggy::petgraph::graph::Edge<Empty>]);
        struct Nodes<'a>(&'a [daggy::petgraph::graph::Node<Release>]);

        impl<'a> Serialize for Edges<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_seq(self.0.iter().map(|edge| (edge.source(), edge.target())))
            }
        }

        impl<'a> Serialize for Nodes<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_seq(self.0.iter().map(|node| &node.weight))
            }
        }

        let mut state = serializer.serialize_struct("Graph", 2)?;
        state.serialize_field("nodes", &Nodes(&self.dag.raw_nodes()))?;
        state.serialize_field("edges", &Edges(&self.dag.raw_edges()))?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    extern crate serde_json;

    use super::*;

    #[test]
    fn serialize_graph() {
        let mut graph = Graph::default();
        let v1 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: Version::new(1, 0, 0),
            payload: String::from("image/1.0.0"),
            metadata: HashMap::new(),
        }));
        let v2 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: Version::new(2, 0, 0),
            payload: String::from("image/2.0.0"),
            metadata: HashMap::new(),
        }));
        let v3 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: Version::new(3, 0, 0),
            payload: String::from("image/3.0.0"),
            metadata: HashMap::new(),
        }));
        graph.dag.add_edge(v1, v2, Empty {}).unwrap();
        graph.dag.add_edge(v2, v3, Empty {}).unwrap();
        graph.dag.add_edge(v1, v3, Empty {}).unwrap();

        assert_eq!(serde_json::to_string(&graph).unwrap(), r#"{"nodes":[{"version":"1.0.0","payload":"image/1.0.0","metadata":{}},{"version":"2.0.0","payload":"image/2.0.0","metadata":{}},{"version":"3.0.0","payload":"image/3.0.0","metadata":{}}],"edges":[[0,1],[1,2],[0,2]]}"#);
    }
}
