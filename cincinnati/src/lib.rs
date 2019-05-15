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
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate commons;
#[macro_use]
extern crate log;
extern crate protobuf;
extern crate toml;
extern crate try_from;
extern crate url;
#[macro_use]
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate smart_default;

pub mod plugins;

use daggy::petgraph::visit::{IntoNodeReferences, NodeRef};
use daggy::{Dag, Walker};
use failure::Error;
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::collections::HashMap;
use std::{collections, fmt};

pub use daggy::WouldCycle;

pub const CONTENT_TYPE: &str = "application/json";
const EXPECT_NODE_WEIGHT: &str = "all exisitng nodes to have a weight (release)";

#[derive(Debug, Default)]
#[cfg_attr(test, derive(Clone))]
pub struct Graph {
    dag: Dag<Release, Empty>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum Release {
    Concrete(ConcreteRelease),
    Abstract(AbstractRelease),
}

impl Release {
    pub fn version(&self) -> &str {
        match self {
            Release::Abstract(release) => &release.version,
            Release::Concrete(release) => &release.version,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct ConcreteRelease {
    pub version: String,
    pub payload: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct AbstractRelease {
    pub version: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ReleaseId(daggy::NodeIndex);

pub struct NextReleases<'a> {
    children: daggy::Children<Release, Empty, daggy::petgraph::graph::DefaultIx>,
    dag: &'a Dag<Release, Empty>,
}

impl<'a> Iterator for NextReleases<'a> {
    type Item = &'a Release;

    fn next(&mut self) -> Option<Self::Item> {
        self.children
            .walk_next(self.dag)
            .map(|(_, i)| self.dag.node_weight(i).expect(EXPECT_NODE_WEIGHT))
    }
}

#[derive(Debug, Clone)]
pub struct Empty;

impl Graph {
    pub fn add_release<R>(&mut self, release: R) -> Result<ReleaseId, Error>
    where
        R: Into<Release>,
    {
        let release = release.into();
        match self.find_by_version(&release.version()) {
            Some(id) => {
                let mut node = self.dag.node_weight_mut(id.0).expect(EXPECT_NODE_WEIGHT);
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

    pub fn find_by_version(&self, version: &str) -> Option<ReleaseId> {
        self.dag
            .node_references()
            .find(|nr| nr.weight().version() == version)
            .map(|nr| ReleaseId(nr.id()))
    }

    pub fn remove_edge(&mut self, from: &ReleaseId, to: &ReleaseId) -> Result<(), Error> {
        if let Some(edge) = self.dag.find_edge(from.0, to.0) {
            self.dag
                .remove_edge(edge)
                .map(|_| ())
                .ok_or_else(|| format_err!("could not remove edge '{:?}'", edge))
        } else {
            bail!("could not find edge from '{:?}' to '{:?}'", from, to);
        }
    }

    pub fn remove_edges(&mut self, indices: HashMap<ReleaseId, ReleaseId>) -> Result<(), Error> {
        indices
            .iter()
            .try_for_each(|(from, to)| self.remove_edge(from, to))
    }

    pub fn add_edge(&mut self, from: &ReleaseId, to: &ReleaseId) -> Result<(), Error> {
        self.dag
            .add_edge(from.0, to.0, Empty {})
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn add_edges(&mut self, indices: HashMap<ReleaseId, ReleaseId>) -> Result<(), Error> {
        indices
            .iter()
            .try_fold((), |_, (from, to)| self.add_edge(&from, &to))
    }

    /// Returns tuples of ReleaseId and its version String for releases for which
    /// filter_fn returns true.
    ///
    /// filter_fn is able to mutate the release as it receives a mutable borrow.
    pub fn find_by_fn_mut<F>(&mut self, mut filter_fn: F) -> Vec<(ReleaseId, String)>
    where
        F: FnMut(&mut Release) -> bool,
    {
        self.dag
            .node_weights_mut()
            .enumerate()
            .filter_map(|(i, nw)| {
                if filter_fn(nw) {
                    Some((
                        ReleaseId(daggy::NodeIndex::from(i as u32)),
                        nw.version().to_string(),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns tuples of ReleaseId and its version String for releases which
    /// match the given metadata key/value pair.
    pub fn find_by_metadata_pair(&self, key: &str, value: &str) -> Vec<(ReleaseId, String)> {
        self.dag
            .node_references()
            .filter(|nr| {
                if let Release::Concrete(release) = nr.weight() {
                    if let Some(found_value) = release.metadata.get(key) {
                        return found_value == value;
                    }
                }
                false
            })
            .map(|nr| (ReleaseId(nr.id()), nr.1.version().to_owned()))
            .collect()
    }

    /// Returns tuples of ReleaseId, its version String, and the value for the given key for releases
    /// which match the given metadata key.
    pub fn find_by_metadata_key(&self, key: &str) -> Vec<(ReleaseId, String, String)> {
        self.dag
            .node_references()
            .filter_map(|nr| {
                if let Release::Concrete(release) = nr.weight() {
                    if let Some(value) = release.metadata.get(key) {
                        return Some((
                            ReleaseId(nr.id()),
                            release.version.to_owned(),
                            value.to_owned(),
                        ));
                    }
                }
                None
            })
            .collect()
    }

    // Returns a mutable reference to the metadata for the given release
    pub fn get_metadata_as_ref_mut(
        &mut self,
        release_id: &ReleaseId,
    ) -> Result<&mut HashMap<String, String>, Error> {
        match self.dag.node_weight_mut(release_id.0) {
            Some(Release::Concrete(release)) => Ok(&mut release.metadata),
            _ => bail!("could not get metadata reference"),
        }
    }

    pub fn next_releases(&self, source: &ReleaseId) -> NextReleases {
        NextReleases {
            children: self.dag.children(source.0),
            dag: &self.dag,
        }
    }

    /// Return the number of releases (nodes) in the graph.
    pub fn releases_count(&self) -> u64 {
        self.dag.node_count() as u64
    }

    /// Removes the nodes with the given ReleaseIds and returns the number of
    /// removed releases.
    ///
    /// The ReleaseIds are sorted and removed in reverse order.
    /// This is required because `daggy::Dag::remove_node()` shifts the NodeIndices
    /// and therefore invalidates all the ones which are higher than the removed one.
    pub fn remove_releases(&mut self, mut to_remove: Vec<ReleaseId>) -> usize {
        to_remove.sort_by(|a, b| {
            use std::cmp::Ordering::*;

            if a.0 < b.0 {
                Less
            } else if a.0 == b.0 {
                Equal
            } else {
                Greater
            }
        });

        to_remove
            .iter()
            .rev()
            .filter(|ri| self.dag.remove_node(ri.0).is_some())
            .count()
    }

    /// Prune the graph from all abstract releases
    ///
    /// Return the number of pruned releases
    pub fn prune_abstract(&mut self) -> usize {
        let to_remove: Vec<daggy::NodeIndex> = self
            .dag
            .node_references()
            .filter_map(|nr| {
                if let Release::Abstract(_) = nr.weight() {
                    Some(nr.0)
                } else {
                    None
                }
            })
            .collect();

        to_remove
            .iter()
            .filter(|ni| self.dag.remove_node(**ni).is_some())
            .count()
    }
}

impl<'a> Deserialize<'a> for Graph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Edges,
            Nodes,
        }

        struct GraphVisitor;

        impl<'de> Visitor<'de> for GraphVisitor {
            type Value = Graph;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Graph")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Graph, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut edges: Option<Vec<(daggy::NodeIndex, daggy::NodeIndex)>> = None;
                let mut nodes: Option<Vec<Release>> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Edges => {
                            if edges.is_some() {
                                return Err(de::Error::duplicate_field("edges"));
                            }
                            edges = Some(map.next_value()?);
                        }
                        Field::Nodes => {
                            if nodes.is_some() {
                                return Err(de::Error::duplicate_field("nodes"));
                            }
                            nodes = Some(map.next_value()?);
                        }
                    }
                }
                let edges = edges.ok_or_else(|| de::Error::missing_field("edges"))?;
                let nodes = nodes.ok_or_else(|| de::Error::missing_field("nodes"))?;
                let mut graph = Graph {
                    dag: Dag::with_capacity(nodes.len(), edges.len()),
                };
                let mut versions = collections::HashSet::with_capacity(nodes.len());
                for node in nodes {
                    // Validate version string is non-empty.
                    if node.version().is_empty() {
                        return Err(de::Error::invalid_value(
                            de::Unexpected::Str(node.version()),
                            &"a non-empty string version",
                        ));
                    }
                    // Validate version string is unique in "nodes" set.
                    if !versions.insert(node.version().to_string()) {
                        return Err(de::Error::invalid_value(
                            de::Unexpected::Str(node.version()),
                            &"a unique string version",
                        ));
                    }
                    graph.dag.add_node(node);
                }
                graph
                    .dag
                    .add_edges(edges.into_iter().map(|(s, t)| (s, t, Empty {})))
                    .map_err(|_| {
                        de::Error::invalid_value(serde::de::Unexpected::StructVariant, &self)
                    })?;
                Ok(graph)
            }
        }

        deserializer.deserialize_struct("Graph", &["nodes", "edges"], GraphVisitor)
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

impl PartialEq for Graph {
    fn eq(&self, other: &Graph) -> bool {
        use daggy::petgraph::visit::IntoNeighbors;

        let asc_order_release_by_version = {
            use std::cmp::Ordering::{self, *};

            |a: &&Release, b: &&Release| -> Ordering {
                if a.version() < b.version() {
                    Less
                } else if a.version() == b.version() {
                    Equal
                } else {
                    Greater
                }
            }
        };

        // Look through all nodes in self
        self.dag.node_references().all(|node_ref| {
            let dag_other = &other.dag;
            let node_index = node_ref.0;
            let release = node_ref.1;

            // For each node in self, look through all nodes in other and find a match
            dag_other
                .node_references()
                .filter(|node_ref_other| {
                    let node_index_other = node_ref_other.0;
                    let release_other = node_ref_other.1;

                    // Ensure the set of neighbors of release and release_other are identical
                    let compare_neighbors = || {
                        let (neighbors_count, neighbors_other_count) = (
                            self.dag.neighbors(node_index).count(),
                            dag_other.neighbors(node_index_other).count(),
                        );

                        if neighbors_count != neighbors_other_count {
                            return false;
                        }

                        let mut neighbors = self
                            .dag
                            .neighbors(node_index)
                            .zip(dag_other.neighbors(node_index_other))
                            .fold(
                                Vec::with_capacity(neighbors_count * 2),
                                |mut neighbors, (neighbor, neighbor_other)| {
                                    neighbors.push(
                                        self.dag.node_weight(neighbor).expect(EXPECT_NODE_WEIGHT),
                                    );
                                    neighbors.push(
                                        dag_other
                                            .node_weight(neighbor_other)
                                            .expect(EXPECT_NODE_WEIGHT),
                                    );
                                    neighbors
                                },
                            );

                        // dedup() requires consecutive sorting
                        neighbors.sort_by(asc_order_release_by_version);
                        neighbors.dedup();

                        neighbors.len() == neighbors_count
                    };

                    release == release_other && compare_neighbors()
                })
                // Ensure each node in self has exactly one matching node in including its neighbors
                .count()
                == 1
        })
    }
}

impl Eq for Graph {}

impl From<plugins::interface::Graph> for Graph {
    fn from(mut graph: plugins::interface::Graph) -> Self {
        let mut graph_converted = Graph::default();

        // Convert nodes
        for node in graph.take_nodes().into_iter() {
            graph_converted
                .dag
                .add_node(Release::Concrete(ConcreteRelease {
                    version: node.version,
                    payload: node.payload,
                    metadata: node.metadata,
                }));
        }

        // Convert edges
        for edge in graph.take_edges().into_iter() {
            graph_converted
                .dag
                .add_edge(
                    daggy::NodeIndex::from(edge.from as u32),
                    daggy::NodeIndex::from(edge.to as u32),
                    Empty {},
                )
                .expect("add_edge");
        }

        graph_converted
    }
}

impl From<Graph> for plugins::interface::Graph {
    fn from(graph: Graph) -> Self {
        use daggy::petgraph::visit::IntoNeighborsDirected;
        use daggy::petgraph::Direction;
        use crate::Release::{Abstract, Concrete};

        let mut nodes_converted: Vec<plugins::interface::Graph_Node> =
            std::vec::Vec::with_capacity(graph.dag.node_count());
        let mut edges_converted: Vec<plugins::interface::Graph_Edge> =
            std::vec::Vec::with_capacity(graph.dag.edge_count());

        for node_reference in graph.dag.node_references() {
            let node_index = node_reference.0;
            let release = node_reference.1;

            // Convert and push node
            let mut node_converted = plugins::interface::Graph_Node::new();
            match release {
                Concrete(concrete_release) => {
                    // TODO(steveeJ): avoid cloning all release content
                    node_converted.set_version(concrete_release.version.clone());
                    node_converted.set_metadata(concrete_release.metadata.clone());
                    node_converted.set_payload(concrete_release.payload.clone());
                }
                Abstract(_) => panic!("found Abstract release type"),
            }
            nodes_converted.push(node_converted);

            // find neighbors and push edges
            for neighbor in graph
                .dag
                .neighbors_directed(node_index, Direction::Outgoing)
            {
                let mut edge_converted = plugins::interface::Graph_Edge::new();
                edge_converted.set_from(node_index.index() as u64);
                edge_converted.set_to(neighbor.index() as u64);
                edges_converted.push(edge_converted);
            }
        }

        let mut graph_converted = plugins::interface::Graph::new();
        graph_converted.set_nodes(nodes_converted.into());
        graph_converted.set_edges(edges_converted.into());

        graph_converted
    }
}

#[cfg(test)]
mod tests {
    extern crate serde_json;

    use super::*;

    pub(crate) fn generate_graph() -> Graph {
        let mut graph = Graph::default();
        let v1 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: String::from("1.0.0"),
            payload: String::from("image/1.0.0"),
            metadata: HashMap::new(),
        }));
        let v2 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: String::from("2.0.0"),
            payload: String::from("image/2.0.0"),
            metadata: HashMap::new(),
        }));
        let v3 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: String::from("3.0.0"),
            payload: String::from("image/3.0.0"),
            metadata: HashMap::new(),
        }));
        graph.dag.add_edge(v1, v2, Empty {}).unwrap();
        graph.dag.add_edge(v2, v3, Empty {}).unwrap();
        graph.dag.add_edge(v1, v3, Empty {}).unwrap();

        graph
    }

    pub(crate) fn generate_custom_graph(
        start: usize,
        count: usize,
        mut metadata: HashMap<usize, HashMap<String, String>>,
        edges: Option<Vec<(usize, usize)>>,
    ) -> Graph {
        let mut graph = Graph::default();

        let nodes: Vec<daggy::NodeIndex> = (start..(start + count))
            .map(|i| {
                let metadata = metadata.remove(&i).unwrap_or(HashMap::new());

                let release = Release::Concrete(ConcreteRelease {
                    version: format!("{}.0.0", i),
                    payload: format!("image/{}.0.0", i),
                    metadata,
                });
                graph.dag.add_node(release)
            })
            .collect();

        assert_eq!(count as u64, graph.releases_count());

        if let Some(edges) = edges {
            for (key, value) in &edges {
                let one = nodes[*key];
                let two = nodes[*value];
                graph.dag.add_edge(one, two, Empty {}).unwrap();
            }
            assert_eq!(edges.len(), graph.dag.edge_count());
        } else {
            for i in 0..(nodes.len() - 1) {
                let one = nodes[i];
                let two = nodes[i + 1];
                graph.dag.add_edge(one, two, Empty {}).unwrap();
            }
        };

        graph
    }

    #[test]
    fn serialize_graph() {
        let graph = generate_graph();
        assert_eq!(serde_json::to_string(&graph).unwrap(), r#"{"nodes":[{"version":"1.0.0","payload":"image/1.0.0","metadata":{}},{"version":"2.0.0","payload":"image/2.0.0","metadata":{}},{"version":"3.0.0","payload":"image/3.0.0","metadata":{}}],"edges":[[0,1],[1,2],[0,2]]}"#);
    }

    #[test]
    fn deserialize_graph() {
        let json = r#"{"nodes":[{"version":"1.0.0","payload":"image/1.0.0","metadata":{}},{"version":"2.0.0","payload":"image/2.0.0","metadata":{}},{"version":"3.0.0","payload":"image/3.0.0","metadata":{}}],"edges":[[0,1],[1,2],[0,2]]}"#;

        let de: Graph = serde_json::from_str(json).unwrap();
        assert_eq!(de.releases_count(), 3);

        let ser = serde_json::to_string(&de).unwrap();
        assert_eq!(ser, json);
    }

    #[test]
    fn test_graph_eq_false_for_unequal_graphs() {
        let graph1 = {
            let mut graph = Graph::default();
            let v1 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
                version: String::from("1.0.0"),
                payload: String::from("image/1.0.0"),
                metadata: HashMap::new(),
            }));
            let v2 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
                version: String::from("2.0.0"),
                payload: String::from("image/2.0.0"),
                metadata: HashMap::new(),
            }));
            graph.dag.add_edge(v1, v2, Empty {}).unwrap();

            graph
        };
        let graph2 = {
            let mut graph = Graph::default();
            let v3 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
                version: String::from("3.0.0"),
                payload: String::from("image/3.0.0"),
                metadata: HashMap::new(),
            }));
            let v2 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
                version: String::from("2.0.0"),
                payload: String::from("image/2.0.0"),
                metadata: HashMap::new(),
            }));
            graph.dag.add_edge(v2, v3, Empty {}).unwrap();

            graph
        };
        assert_ne!(graph1, graph2);
    }

    #[test]
    fn test_graph_eq_true_for_equal_graphs() {
        assert_eq!(generate_graph(), generate_graph())
    }

    #[test]
    fn test_graph_eq_is_agnostic_to_node_and_edge_order() {
        let r1 = Release::Concrete(ConcreteRelease {
            version: String::from("1.0.0"),
            payload: String::from("image/1.0.0"),
            metadata: HashMap::new(),
        });
        let r2 = Release::Concrete(ConcreteRelease {
            version: String::from("2.0.0"),
            payload: String::from("image/2.0.0"),
            metadata: HashMap::new(),
        });

        let r3 = Release::Concrete(ConcreteRelease {
            version: String::from("3.0.0"),
            payload: String::from("image/3.0.0"),
            metadata: HashMap::new(),
        });

        let graph1 = {
            let mut graph = Graph::default();
            let v1 = graph.dag.add_node(r1.clone());
            let v2 = graph.dag.add_node(r2.clone());
            let v3 = graph.dag.add_node(r3.clone());
            graph.dag.add_edge(v1, v2, Empty {}).unwrap();
            graph.dag.add_edge(v1, v3, Empty {}).unwrap();
            graph.dag.add_edge(v2, v3, Empty {}).unwrap();

            graph
        };
        let graph2 = {
            let mut graph = Graph::default();
            let v3 = graph.dag.add_node(r3.clone());
            let v2 = graph.dag.add_node(r2.clone());
            let v1 = graph.dag.add_node(r1.clone());
            graph.dag.add_edge(v2, v3, Empty {}).unwrap();
            graph.dag.add_edge(v1, v2, Empty {}).unwrap();
            graph.dag.add_edge(v1, v3, Empty {}).unwrap();

            graph
        };
        assert_eq!(graph1, graph2);
    }

    #[test]
    fn roundtrip_conversion_from_graph_via_plugin_interface() {
        let graph_plugin_interface: plugins::interface::Graph = generate_graph().into();
        let graph_native_converted: Graph = graph_plugin_interface.into();

        assert_eq!(generate_graph(), graph_native_converted);
    }

    fn get_test_metadata_fn_mut(
        key_prefix: &str,
        key_suffix: &str,
    ) -> HashMap<usize, HashMap<String, String>> {
        [
            (
                0,
                [(
                    format!("{}.{}", &key_prefix, &key_suffix),
                    String::from("A, C"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            (
                1,
                [(
                    format!("{}.{}", &key_prefix, &key_suffix),
                    String::from("A, C"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            (
                2,
                [(
                    format!("{}.{}", &key_prefix, &key_suffix),
                    String::from("B, C"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            (
                3,
                [(
                    format!("{}.{}", &key_prefix, &key_suffix),
                    String::from("B, C"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            (4, [].iter().cloned().collect()),
        ]
        .iter()
        .cloned()
        .collect()
    }

    #[test]
    fn find_by_fn_mut_ensure_find_all() {
        let metadata = get_test_metadata_fn_mut("prefix", "suffix");
        let mut graph = generate_custom_graph(0, 4, metadata, Some(vec![]));

        let expected = vec![(0, "0.0.0"), (1, "1.0.0"), (2, "2.0.0"), (3, "3.0.0")]
            .into_iter()
            .map(|(id, version)| {
                (
                    ReleaseId(daggy::NodeIndex::from(id as u32)),
                    version.to_string(),
                )
            })
            .collect::<Vec<(ReleaseId, String)>>();
        let result = graph.find_by_fn_mut(|_| true);
        assert_eq!(expected, result);
    }

    #[test]
    fn find_by_fn_mut_ensure_mutate_metadata() {
        let (prefix, suffix) = ("prefix", "suffix");
        let metadata = get_test_metadata_fn_mut(&prefix, &suffix);
        let mut graph = generate_custom_graph(0, 4, metadata, Some(vec![]));

        let expected = vec![(0, "0.0.0"), (1, "1.0.0"), (2, "2.0.0"), (3, "3.0.0")]
            .into_iter()
            .map(|(id, version)| {
                (
                    ReleaseId(daggy::NodeIndex::from(id as u32)),
                    version.to_string(),
                )
            })
            .collect::<Vec<(ReleaseId, String)>>();

        let metadata_key = format!("{}.{}", &prefix, &suffix);
        let expected_metadata_value = "changed";

        let result = graph.find_by_fn_mut(|release| match release {
            Release::Concrete(concrete_release) => {
                *concrete_release.metadata.get_mut(&metadata_key).unwrap() =
                    expected_metadata_value.to_string();
                true
            }
            _ => true,
        });

        assert_eq!(expected, result);

        result.into_iter().for_each(|(release_id, _)| {
            assert_eq!(
                graph
                    .get_metadata_as_ref_mut(&release_id)
                    .unwrap()
                    .get(&metadata_key)
                    .unwrap(),
                expected_metadata_value
            )
        });
    }
}
