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

#[macro_use]
extern crate serde_derive;

#[macro_use]
pub mod plugins;

use commons::prelude_errors::*;
use daggy::petgraph::visit::{IntoNodeReferences, NodeRef};
use daggy::{Dag, EdgeIndex, Walker};
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::{collections, fmt};

pub use daggy::{self, WouldCycle};

pub const CONTENT_TYPE: &str = "application/json";
const EXPECT_NODE_WEIGHT: &str = "all exisitng nodes to have a weight (release)";

#[cfg(not(any(test, feature = "test")))]
pub use std::collections::HashMap as MapImpl;

#[cfg(not(any(test, feature = "test")))]
pub use std::collections::HashSet as SetImpl;

#[cfg(any(test, feature = "test"))]
pub use std::collections::BTreeMap as MapImpl;

#[cfg(any(test, feature = "test"))]
pub use std::collections::BTreeSet as SetImpl;

/// Graph type which stores `Release` as node-weights and `Empty` as edge-weights.
#[derive(Debug, Default)]
#[cfg_attr(any(test, feature = "test"), derive(Clone))]
pub struct Graph {
    dag: Dag<Release, Empty>,
}

/// Wrapper enum for the concrete and abstract release types.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum Release {
    Concrete(ConcreteRelease),
    Abstract(AbstractRelease),
}

impl Release {
    /// Return the version string of a given `Release`.
    pub fn version(&self) -> &str {
        match self {
            Release::Abstract(release) => &release.version,
            Release::Concrete(release) => &release.version,
        }
    }

    /// Get a mutable borrow of the release metadata if any
    pub fn get_metadata_mut(&mut self) -> Option<&mut MapImpl<String, String>> {
        match self {
            Release::Abstract(_) => None,
            Release::Concrete(release) => Some(&mut release.metadata),
        }
    }

    /// Returns the `manifestref` of a given `Release`
    pub fn manifestref(&self) -> Result<&String, Error> {
        let digestkey = String::from("io.openshift.upgrades.graph.release.manifestref");
        match self {
            Release::Concrete(release) => {
                let digest = release.metadata.get(&digestkey);
                Ok(digest.map(|d| d).unwrap())
            }
            _ => bail!("could not get manifest reference"),
        }
    }
}

/// Type to represent a Release with all its information.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct ConcreteRelease {
    pub version: String,
    pub payload: String,
    pub metadata: MapImpl<String, String>,
}

/// Abtract release only storing a version.
///
/// It can be used for adding an edge between an existing and a non-existing
/// release, and is expected to later be filled up with a `ConcreteRelease` once
/// the graph is completed.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct AbstractRelease {
    pub version: String,
}

/// Abstraction over a node in the graph representing a `Release`
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ReleaseId(daggy::NodeIndex);

/// Can be used to iterate over all direct children of the given release.
///
/// See the `next_releases` method for more information.
pub struct NextReleases<'a> {
    children: daggy::Children<Release, Empty, daggy::petgraph::graph::DefaultIx>,
    dag: &'a Dag<Release, Empty>,
}

impl<'a> Iterator for NextReleases<'a> {
    type Item = (daggy::EdgeIndex, daggy::NodeIndex, &'a Release);

    fn next(&mut self) -> Option<Self::Item> {
        self.children
            .walk_next(self.dag)
            .map(|(edge_index, node_index)| {
                (
                    edge_index,
                    node_index,
                    self.dag.node_weight(node_index).expect(EXPECT_NODE_WEIGHT),
                )
            })
    }
}

/// Can be used to iterate over all direct parents of the given release.
///
/// See the `previous_releases` method for more information.
pub struct PreviousReleases<'a> {
    parents: daggy::Parents<Release, Empty, daggy::petgraph::graph::DefaultIx>,
    dag: &'a Dag<Release, Empty>,
}

impl<'a> Iterator for PreviousReleases<'a> {
    type Item = (daggy::EdgeIndex, daggy::NodeIndex, &'a Release);

    fn next(&mut self) -> Option<Self::Item> {
        self.parents
            .walk_next(self.dag)
            .map(|(edge_index, node_index)| {
                (
                    edge_index,
                    node_index,
                    self.dag.node_weight(node_index).expect(EXPECT_NODE_WEIGHT),
                )
            })
    }
}

/// Dummy type used as edge-weights inside `Graph`.
#[derive(Debug, Clone)]
pub struct Empty;

/// Errors that can be returned by the methods in this library
pub mod errors {
    use commons::prelude_errors::*;

    /// Edge already exists
    #[derive(Debug, Fail, Eq, PartialEq)]
    #[error("edge from {:?} to {:?} already exists", from, to)]
    pub struct EdgeAlreadyExists {
        pub(crate) from: String,
        pub(crate) to: String,
    }

    /// Edge doesn't exist
    #[derive(Debug, Fail, Eq, PartialEq)]
    #[error("edge from '{:?}' to '{:?}' doesn't exist", from, to)]
    pub struct EdgeDoesntExist {
        pub(crate) from: String,
        pub(crate) to: String,
    }

    /// Missing node weight
    #[derive(Debug, Fail, Eq, PartialEq)]
    #[error("NodeWeight with index {} is missing", 0)]
    pub struct NodeWeightMissing(pub(crate) usize);
}

impl Graph {
    /// Add a release to the graph.
    ///
    /// Fails if the version already exists.
    pub fn add_release<R>(&mut self, release: R) -> Result<ReleaseId, Error>
    where
        R: Into<Release>,
    {
        let release = release.into();
        match self.find_by_version(&release.version()) {
            Some(id) => {
                let node = self.dag.node_weight_mut(id.0).expect(EXPECT_NODE_WEIGHT);
                if let Release::Concrete(_) = node {
                    // check if release digest and node digest are same
                    if release.manifestref().unwrap() != node.manifestref().unwrap() {
                        bail!(
                            "mismatched manifest ref for concrete release {}: {}, {}",
                            release.version(),
                            release.manifestref().unwrap(),
                            node.manifestref().unwrap()
                        )
                    }
                }
                *node = release;
                Ok(id)
            }
            None => Ok(ReleaseId(self.dag.add_node(release))),
        }
    }

    /// Add a transition (edge) from `source` to `target`.
    ///
    /// Fails with the `WouldCycle` error if the new edge would lead to a cycle.
    pub fn add_edge(&mut self, from: &ReleaseId, to: &ReleaseId) -> Result<EdgeIndex, Error> {
        let from_release: String = self.find_by_releaseid(from)?.version().to_string();
        let to_release: String = self.find_by_releaseid(to)?.version().to_string();
        if self.dag.find_edge(from.0, to.0).is_some() {
            return Err(Error::from(errors::EdgeAlreadyExists {
                from: from_release,
                to: to_release,
            }));
        }

        self.dag
            .add_edge(from.0, to.0, Empty {})
            .map_err(Into::into)
    }

    /// Add edges for all given key/value pairs of releases.
    pub fn add_edges(&mut self, indices: MapImpl<ReleaseId, ReleaseId>) -> Result<(), Error> {
        indices
            .iter()
            .try_fold((), |_, (from, to)| self.add_edge(&from, &to).map(|_| ()))
    }

    /// Returns a Some(ReleaseId) if the version exists in the graph, None otherwise.
    pub fn find_by_version(&self, version: &str) -> Option<ReleaseId> {
        self.dag
            .node_references()
            .find(|nr| nr.weight().version() == version)
            .map(|nr| ReleaseId(nr.id()))
    }

    /// Returns a Release for the given &ReleaseId
    pub fn find_by_releaseid(&self, id: &ReleaseId) -> Fallible<&Release> {
        self.dag
            .node_weight(id.0)
            .ok_or_else(move || format_err!("could not find Release with id: {:?}", id))
    }

    /// Removes the directed edge between the given releases.
    pub fn remove_edge(&mut self, from: &ReleaseId, to: &ReleaseId) -> Result<(), Error> {
        if let Some(edge) = self.dag.find_edge(from.0, to.0) {
            self.dag
                .remove_edge(edge)
                .map(|_| ())
                .ok_or_else(|| format_err!("could not remove edge '{:?}'", edge))
        } else {
            Err(Error::from(errors::EdgeDoesntExist {
                from: self.find_by_releaseid(from)?.version().to_string(),
                to: self.find_by_releaseid(to)?.version().to_string(),
            }))
        }
    }

    /// Remove the directed edges given by the key/value pairs of releases.
    pub fn remove_edges(&mut self, indices: MapImpl<ReleaseId, ReleaseId>) -> Result<(), Error> {
        indices
            .iter()
            .try_for_each(|(from, to)| self.remove_edge(from, to))
    }

    /// Remove the edge with the given index.
    ///
    /// Fails if the edge wasn't found and thus couldn't be removed.
    pub fn remove_edge_by_index(&mut self, index: daggy::EdgeIndex) -> Result<(), Error> {
        match self.dag.remove_edge(index) {
            Some(_) => Ok(()),
            None => bail!("could not remove edge with index {:?}", index),
        }
    }

    /// Remove the edges with the given indices.
    ///
    /// Stops and fails at the first edge which couldn't be removed.
    pub fn remove_edges_by_index(&mut self, indices: &[daggy::EdgeIndex]) -> Result<(), Error> {
        // Reverse sort the indices to start removing from high numbers.
        // This is required because removing lower numbers first breaks higher number indices.
        let mut indices_sorted = indices.to_vec();
        indices_sorted.sort_by(|a, b| b.cmp(a));

        indices_sorted
            .iter()
            .try_for_each(|ei| self.remove_edge_by_index(*ei))
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

    /// Returns a mutable reference to the metadata for the given release.
    pub fn get_metadata_as_ref_mut(
        &mut self,
        release_id: &ReleaseId,
    ) -> Result<&mut MapImpl<String, String>, Error> {
        match self.dag.node_weight_mut(release_id.0) {
            Some(Release::Concrete(release)) => Ok(&mut release.metadata),
            _ => bail!("could not get metadata reference"),
        }
    }

    /// Returns `NextReleases` for the given release.
    ///
    /// `NextReleases` can be used to iterate over all direct children of the given release.
    pub fn next_releases(&self, source: &ReleaseId) -> NextReleases {
        NextReleases {
            children: self.dag.children(source.0),
            dag: &self.dag,
        }
    }

    /// Returns `PreviousReleases` for the given release.
    ///
    /// `PreviousReleases` can be used to iterate over all direct parents of the given release.
    pub fn previous_releases(&self, source: &ReleaseId) -> PreviousReleases {
        PreviousReleases {
            parents: self.dag.parents(source.0),
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

        self.remove_nodes(to_remove.into_iter().map(|ri| ri.0).collect())
    }

    /// Removes the nodes with the given NodeIndex and returns the number of
    /// removed nodes.
    pub fn remove_nodes(&mut self, to_remove: Vec<daggy::NodeIndex>) -> usize {
        to_remove
            .into_iter()
            .rev()
            .filter(|ni| self.dag.remove_node(*ni).is_some())
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

    /// Iterates over all releases mutably
    ///
    /// f is able to mutate the release as it receives a mutable borrow.
    pub fn iter_releases_mut<F>(&mut self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(&mut Release) -> Result<(), Error>,
    {
        self.dag
            .node_weights_mut()
            .try_for_each(|mut nw| f(&mut nw))
    }

    /// Get the edges expressed as version -> versions; optionally include edges from/to `Release::Abstract`.
    #[cfg(any(test, feature = "test"))]
    pub fn get_edges(
        &self,
        include_abstract: bool,
    ) -> Result<MapImpl<String, SetImpl<String>>, Error> {
        let mut edges: MapImpl<String, SetImpl<String>> = Default::default();

        self.dag
            .raw_edges()
            .iter()
            .try_for_each(|edge| -> Result<(), Error> {
                match (
                    self.dag.node_weight(edge.source()),
                    self.dag.node_weight(edge.target()),
                ) {
                    (Some(source), Some(target)) => {
                        let source_version = source.version().to_string();
                        let target_version = target.version().to_string();

                        if include_abstract
                            || match (source, target) {
                                (Release::Concrete(_), Release::Concrete(_)) => true,
                                _ => false,
                            }
                        {
                            edges
                                .entry(source_version)
                                .or_default()
                                .insert(target_version);
                        }

                        Ok(())
                    }
                    (None, _) => Err(errors::NodeWeightMissing(edge.source().index())),
                    (_, None) => Err(errors::NodeWeightMissing(edge.target().index())),
                }
                .map_err(Error::from)?;

                Ok(())
            })?;

        Ok(edges)
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

#[cfg(any(test, feature = "test"))]
impl PartialEq for Graph {
    fn eq(&self, other: &Graph) -> bool {
        let mut releases = self
            .dag
            .node_references()
            .map(|node_ref| node_ref.1)
            .collect::<Vec<&Release>>();
        releases.sort();

        let mut releases_other = other
            .dag
            .node_references()
            .map(|node_ref| node_ref.1)
            .collect::<Vec<&Release>>();
        releases_other.sort();

        if releases != releases_other {
            return false;
        }

        let edges = if let Ok(edges) = self.get_edges(true) {
            edges
        } else {
            return false;
        };

        let edges_other = if let Ok(edges) = other.get_edges(true) {
            edges
        } else {
            return false;
        };

        if edges != edges_other {
            return false;
        }

        true
    }
}

#[cfg(any(test, feature = "test"))]
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
                    metadata: node.metadata.into_iter().collect(),
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
        use crate::Release::{Abstract, Concrete};
        use daggy::petgraph::visit::IntoNeighborsDirected;
        use daggy::petgraph::Direction;

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
                    node_converted
                        .set_metadata(concrete_release.metadata.clone().into_iter().collect());
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

#[cfg(any(test, feature = "test"))]
pub mod testing {
    use super::*;

    pub fn generate_graph() -> Graph {
        let mut graph = Graph::default();
        let v1 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: String::from("1.0.0"),
            payload: String::from("image/1.0.0"),
            metadata: MapImpl::new(),
        }));
        let v2 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: String::from("2.0.0"),
            payload: String::from("image/2.0.0"),
            metadata: MapImpl::new(),
        }));
        let v3 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
            version: String::from("3.0.0"),
            payload: String::from("image/3.0.0"),
            metadata: MapImpl::new(),
        }));
        graph.dag.add_edge(v1, v2, Empty {}).unwrap();
        graph.dag.add_edge(v2, v3, Empty {}).unwrap();
        graph.dag.add_edge(v1, v3, Empty {}).unwrap();

        graph
    }

    pub type TestMetadata = Vec<(usize, MapImpl<String, String>)>;
    pub type TestEdges = Vec<(usize, usize)>;

    #[derive(Debug, Clone)]
    pub struct TestGraphBuilder {
        image: String,
        metadata: TestMetadata,
        edges: Option<TestEdges>,
        version_template: String,
        enable_payload_suffix: bool,
    }

    impl Default for TestGraphBuilder {
        fn default() -> Self {
            TestGraphBuilder {
                image: "image".to_string(),
                metadata: Default::default(),
                edges: None,
                version_template: "{{i}}.0.0".to_string(),
                enable_payload_suffix: false,
            }
        }
    }

    impl TestGraphBuilder {
        pub fn new() -> Self {
            Default::default()
        }

        pub fn with_image(mut self, image: &str) -> Self {
            self.image = image.to_string();
            self
        }

        pub fn with_metadata(mut self, metadata: TestMetadata) -> Self {
            self.metadata = metadata;
            self
        }

        pub fn with_edges(mut self, edges: Option<TestEdges>) -> Self {
            self.edges = edges;
            self
        }

        pub fn with_version_template(mut self, version_template: &str) -> Self {
            self.version_template = version_template.to_string();
            self
        }

        pub fn enable_payload_suffix(mut self, enable: bool) -> Self {
            self.enable_payload_suffix = enable;
            self
        }

        pub fn build(self) -> Graph {
            let mut graph = Graph::default();

            let nodes: Vec<daggy::NodeIndex> = self
                .clone()
                .metadata
                .into_iter()
                .map(|(i, mut metadata)| {
                    let version_unsuffixed = self.version_template.replace("{{i}}", &i.to_string());
                    let version_suffix = metadata.remove("version_suffix").unwrap_or_default();

                    let version = format!("{}{}", version_unsuffixed, version_suffix);
                    let payload = format!(
                        "{}:{}",
                        &self.image,
                        if self.enable_payload_suffix {
                            &version
                        } else {
                            &version_unsuffixed
                        }
                    );

                    let release = Release::Concrete(ConcreteRelease {
                        version,
                        payload,
                        metadata,
                    });
                    graph.dag.add_node(release)
                })
                .collect();

            assert_eq!(nodes.len() as u64, graph.releases_count());

            if let Some(edges) = self.edges {
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
    }

    pub fn generate_custom_graph(
        image: &str,
        metadata: TestMetadata,
        edges: Option<TestEdges>,
    ) -> Graph {
        TestGraphBuilder::new()
            .with_image(image)
            .with_metadata(metadata)
            .with_edges(edges)
            .build()
    }

    impl std::cmp::PartialOrd for Release {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.version().cmp(other.version()))
        }
    }

    impl From<&Graph> for std::collections::BTreeSet<Release> {
        fn from(graph: &Graph) -> Self {
            let mut graph = graph.clone();
            let mut set = Self::new();
            let _ = graph.iter_releases_mut(|node| {
                set.insert(node.clone());
                Ok(())
            });
            set
        }
    }

    impl std::cmp::Ord for Release {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.version().cmp(other.version())
        }
    }

    /// Settings for the `compare_graphs_verbose` fn
    #[derive(Default)]
    pub struct CompareGraphsVerboseSettings<'a> {
        pub unwanted_metadata_keys: &'a [&'a str],
        pub payload_replace_sha_by_tag_left: bool,
        pub payload_replace_sha_by_tag_right: bool,
        pub payload_remove_registry_and_repo: bool,
    }

    /// Compares two Graphs and gives a verbose error if not equal.
    pub fn compare_graphs_verbose(
        mut left: Graph,
        mut right: Graph,
        settings: CompareGraphsVerboseSettings,
    ) -> Fallible<()> {
        let removed_keys_left = remove_release_metadata(&mut left, settings.unwanted_metadata_keys);
        let removed_keys_right =
            remove_release_metadata(&mut right, settings.unwanted_metadata_keys);

        if settings.payload_replace_sha_by_tag_left {
            payload_replace_sha_by_tag_fn(&mut left)?;
        }

        if settings.payload_replace_sha_by_tag_right {
            payload_replace_sha_by_tag_fn(&mut right)?;
        }

        if settings.payload_remove_registry_and_repo {
            payload_remove_registry_and_repo(&mut left)?;
            payload_remove_registry_and_repo(&mut right)?;
        }

        if left == right {
            return Ok(());
        }

        let mut output: Vec<String> = vec![];
        output.push("Graphs differ! Showing differences from left to right".into());
        output.push("-----------------------------------------------------".into());

        let edges_left = left.get_edges(false)?;
        let edges_right = right.get_edges(false)?;
        output.push("edges: ".into());
        output.push(
            prettydiff::diff_lines(
                &serde_json::to_string_pretty(&edges_left)?,
                &serde_json::to_string_pretty(&edges_right)?,
            )
            .format(),
        );

        let releases_left: std::collections::BTreeSet<Release> = (&left).into();
        let releases_right: std::collections::BTreeSet<Release> = (&right).into();
        output.push("nodes:".into());
        output.push(
            prettydiff::diff_lines(
                Box::leak(Box::new(serde_json::to_string_pretty(&releases_left)?)),
                Box::leak(Box::new(serde_json::to_string_pretty(&releases_right)?)),
            )
            .format(),
        );

        if !settings.unwanted_metadata_keys.is_empty() {
            output.push("removed metadata:".into());
            output.push(
                prettydiff::diff_lines(
                    &serde_json::to_string_pretty(&removed_keys_left)?,
                    &serde_json::to_string_pretty(&removed_keys_right)?,
                )
                .format(),
            );
        }

        bail!(output.join("\n"));
    }

    /// Removes the metadata given by the keys and returns it.
    pub fn remove_release_metadata(
        graph: &mut Graph,
        keys: &[&str],
    ) -> MapImpl<String, MapImpl<String, String>> {
        let mut removed_metadata = MapImpl::<String, MapImpl<String, String>>::new();

        let _ = graph.iter_releases_mut(|mut release| {
            for key in keys {
                let key = (*key).to_string();

                if let Release::Concrete(concrete_release) = &mut release {
                    if let Some(removed_value) = concrete_release.metadata.remove(&key) {
                        removed_metadata
                            .entry(release.version().to_string())
                            .or_default()
                            .insert(key, removed_value);
                    }
                }
            }

            // we don't care about this
            Ok(())
        });

        removed_metadata
    }

    /// Replace the digests by versioned tags in the release payload strings.
    pub fn payload_replace_sha_by_tag_fn(graph: &mut Graph) -> Fallible<()> {
        graph
            .iter_releases_mut(|ref mut release| {
                match release {
                    Release::Concrete(ref mut release) => {
                        // replace digest by tag to match expectency
                        let version = release.version.to_string();
                        let source_front = release.payload.split('@').next().ok_or_else(|| {
                            Error::msg(format!("invalid version string {:?}", version))
                        })?;
                        release.payload = format!("{}:{}", source_front, version);

                        Ok(())
                    }
                    Release::Abstract(ar) => panic!("unexpected Abstract release: {:?}", &ar),
                }
            })
            .context("replacing the sha by the tag in the payload string")
    }

    /// Remove the registry and the repository in release payload strings.
    pub fn payload_remove_registry_and_repo(graph: &mut Graph) -> Fallible<()> {
        graph
            .iter_releases_mut(|ref mut release| {
                match release {
                    Release::Concrete(ref mut release) => {
                        // replace digest by tag to match expectency
                        let version = release.version.to_string();
                        release.payload = version;

                        Ok(())
                    }
                    Release::Abstract(ar) => panic!("unexpected Abstract release: {:?}", &ar),
                }
            })
            .context("removing registry and repo from the payload string")
    }
}

#[cfg(test)]
mod tests {
    extern crate serde_json;

    use super::testing::*;
    use super::*;

    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn serialize_graph() {
        let graph = generate_graph();
        assert_eq!(
            serde_json::to_string(&graph).unwrap(),
            r#"{"nodes":[{"version":"1.0.0","payload":"image/1.0.0","metadata":{}},{"version":"2.0.0","payload":"image/2.0.0","metadata":{}},{"version":"3.0.0","payload":"image/3.0.0","metadata":{}}],"edges":[[0,1],[1,2],[0,2]]}"#
        );
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
                metadata: MapImpl::new(),
            }));
            let v2 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
                version: String::from("2.0.0"),
                payload: String::from("image/2.0.0"),
                metadata: MapImpl::new(),
            }));
            graph.dag.add_edge(v1, v2, Empty {}).unwrap();

            graph
        };
        let graph2 = {
            let mut graph = Graph::default();
            let v3 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
                version: String::from("3.0.0"),
                payload: String::from("image/3.0.0"),
                metadata: MapImpl::new(),
            }));
            let v2 = graph.dag.add_node(Release::Concrete(ConcreteRelease {
                version: String::from("2.0.0"),
                payload: String::from("image/2.0.0"),
                metadata: MapImpl::new(),
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
            metadata: MapImpl::new(),
        });
        let r2 = Release::Concrete(ConcreteRelease {
            version: String::from("2.0.0"),
            payload: String::from("image/2.0.0"),
            metadata: MapImpl::new(),
        });

        let r3 = Release::Concrete(ConcreteRelease {
            version: String::from("3.0.0"),
            payload: String::from("image/3.0.0"),
            metadata: MapImpl::new(),
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
    fn test_graph_eq_detects_exceeding_nodes() {
        let r1 = Release::Concrete(ConcreteRelease {
            version: String::from("1.0.0"),
            payload: String::from("image/1.0.0"),
            metadata: MapImpl::new(),
        });
        let r2 = Release::Concrete(ConcreteRelease {
            version: String::from("2.0.0"),
            payload: String::from("image/2.0.0"),
            metadata: MapImpl::new(),
        });

        let r3 = Release::Concrete(ConcreteRelease {
            version: String::from("3.0.0"),
            payload: String::from("image/3.0.0"),
            metadata: MapImpl::new(),
        });

        let graph1 = {
            let mut graph = Graph::default();
            let v1 = graph.dag.add_node(r1.clone());
            let v2 = graph.dag.add_node(r2.clone());
            graph.dag.add_edge(v1, v2, Empty {}).unwrap();

            graph
        };
        let graph2 = {
            let mut graph = Graph::default();
            let v1 = graph.dag.add_node(r1.clone());
            let v2 = graph.dag.add_node(r2.clone());
            let _ = graph.dag.add_node(r3.clone());
            graph.dag.add_edge(v1, v2, Empty {}).unwrap();

            graph
        };
        assert_ne!(graph1, graph2);
    }

    #[test]
    fn roundtrip_conversion_from_graph_via_plugin_interface() {
        let graph_plugin_interface: plugins::interface::Graph = generate_graph().into();
        let graph_native_converted: Graph = graph_plugin_interface.into();

        assert_eq!(generate_graph(), graph_native_converted);
    }

    fn get_test_metadata_fn_mut(key_prefix: &str, key_suffix: &str) -> TestMetadata {
        vec![
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
        ]
    }

    #[test]
    fn find_by_fn_mut_ensure_find_all() {
        let metadata = get_test_metadata_fn_mut("prefix", "suffix");
        let mut graph = generate_custom_graph("image", metadata, Some(vec![]));

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
        let mut graph = generate_custom_graph("image", metadata, Some(vec![]));

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

    #[test]
    fn next_releases_yields_all_direct_children() -> TestResult<()> {
        use std::collections::HashSet;
        let n = 6;
        let graph = generate_custom_graph(
            "image",
            (0..n).map(|i| (i, Default::default())).collect(),
            Some(vec![(0, 1), (1, 2), (1, 3), (1, 4), (2, 5), (3, 5), (4, 5)]),
        );

        let expected: HashSet<String> = generate_custom_graph(
            "image",
            (2..=4).map(|i| (i, Default::default())).collect(),
            None,
        )
        .find_by_fn_mut(|_| true)
        .into_iter()
        .map(|(_, version)| version)
        .collect();

        let anchor_version = "1.0.0";

        let v3 = graph
            .find_by_version(anchor_version)
            .ok_or_else(|| format!("couldn't find version {}", anchor_version))?;

        let result: HashSet<String> = graph
            .next_releases(&v3)
            .map(|(_, _, r)| r.version())
            .map(ToString::to_string)
            .collect();

        assert_eq!(expected, result);
        assert_eq!(result.len(), 3, "expected 3 results");

        Ok(())
    }

    #[test]
    fn previous_releases_yields_all_direct_parents() -> TestResult<()> {
        use std::collections::HashSet;
        let n = 6;
        let graph = generate_custom_graph(
            "image",
            (0..n).map(|i| (i, Default::default())).collect(),
            Some(vec![(0, 1), (1, 4), (2, 4), (3, 4), (4, 5)]),
        );

        let expected: HashSet<String> = generate_custom_graph(
            "image",
            (1..=3).map(|i| (i, Default::default())).collect(),
            None,
        )
        .find_by_fn_mut(|_| true)
        .into_iter()
        .map(|(_, version)| version)
        .collect();

        let anchor_version = "4.0.0";

        let v3 = graph
            .find_by_version(anchor_version)
            .ok_or_else(|| format!("couldn't find version {}", anchor_version))?;

        let result: HashSet<String> = graph
            .previous_releases(&v3)
            .map(|(_, _, r)| r.version())
            .map(ToString::to_string)
            .collect();

        assert_eq!(expected, result);
        assert_eq!(result.len(), 3, "expected 3 results");

        Ok(())
    }
}
