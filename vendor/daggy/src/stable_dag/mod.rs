//! This module includes the implementation of the **StableDag** data structure. The **StableDag**
//! has a similar functionality to the **Dag** data structure, but it does not invalidate node
//! indices when a node is removed.

use crate::walker;
use crate::{Dag, WouldCycle};
use petgraph as pg;
use petgraph::algo::{has_path_connecting, DfsSpace};
use petgraph::stable_graph::{DefaultIx, GraphIndex, IndexType, StableDiGraph};
use petgraph::visit::{
    GetAdjacencyMatrix, GraphBase, GraphProp, IntoEdgeReferences, IntoEdges, IntoEdgesDirected,
    IntoNeighbors, IntoNeighborsDirected, IntoNodeIdentifiers, IntoNodeReferences,
    NodeCompactIndexable, NodeCount, NodeIndexable, Visitable,
};
use petgraph::IntoWeightedEdge;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

// Petgraph re-exports.
pub use petgraph::graph::{EdgeIndex, EdgeWeightsMut, NodeIndex, NodeWeightsMut};
pub use petgraph::visit::Walker;

#[cfg(feature = "serde-1")]
mod serde;

/// An iterator yielding all edges to/from some node.
pub type Edges<'a, E, Ix> = pg::stable_graph::Edges<'a, E, pg::Directed, Ix>;

/// A Directed acyclic graph (DAG) data structure.
///
/// StableDag is a thin wrapper around petgraph's `StableGraph` data structure, providing a refined
/// API for dealing specifically with DAGs.
///
/// Note: The following documentation is adapted from petgraph's [**StableGraph** documentation][1].
///
/// **StableDag** is parameterized over the node weight **N**, edge weight **E** and index type
/// **Ix**.
///
/// **NodeIndex** is a type that acts as a reference to nodes, but these are only stable across
/// certain operations. **StableDag++ keeps all indices stable even when removing nodes/edges.
///
/// The **Ix** parameter is u32 by default. The goal is that you can ignore this parameter
/// completely unless you need a very large **StableDag** -- then you can use usize.
///
/// The **StableDag** also offers methods for accessing the underlying **StableGraph**, which can be
/// useful for taking advantage of petgraph's various graph-related algorithms.
///
///
/// [1]: petgraph::stable_graph::StableGraph
#[derive(Clone, Debug)]
pub struct StableDag<N, E, Ix: IndexType = DefaultIx> {
    graph: StableDiGraph<N, E, Ix>,
    cycle_state: DfsSpace<NodeIndex<Ix>, <StableDiGraph<N, E, Ix> as Visitable>::Map>,
}

/// A **Walker** type that can be used to step through the children of some parent node.
pub struct Children<N, E, Ix: IndexType> {
    walk_edges: pg::stable_graph::WalkNeighbors<Ix>,
    _node: PhantomData<N>,
    _edge: PhantomData<E>,
}

/// A **Walker** type that can be used to step through the parents of some child node.
pub struct Parents<N, E, Ix: IndexType> {
    walk_edges: pg::stable_graph::WalkNeighbors<Ix>,
    _node: PhantomData<N>,
    _edge: PhantomData<E>,
}

/// An iterator yielding multiple `EdgeIndex`s, returned by the `StableGraph::add_edges` method.
pub struct EdgeIndices<Ix: IndexType> {
    indices: ::std::ops::Range<usize>,
    _phantom: PhantomData<Ix>,
}

/// An alias to simplify the **Recursive** **Walker** type returned by **StableDag**.
pub type RecursiveWalk<N, E, Ix, F> = walker::Recursive<StableDag<N, E, Ix>, F>;

impl<N, E, Ix> StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    /// Create a new, empty `StableDag`.
    pub fn new() -> Self {
        Self::with_capacity(1, 1)
    }

    /// Create a new `StableDag` with estimated capacity for its node and edge Vecs.
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        StableDag {
            graph: StableDiGraph::with_capacity(nodes, edges),
            cycle_state: DfsSpace::default(),
        }
    }

    /// Create a `StableDag` from an iterator yielding edges.
    ///
    /// Node weights `N` are set to default values.
    ///
    /// `Edge` weights `E` may either be specified in the list, or they are filled with default
    /// values.
    ///
    /// Nodes are inserted automatically to match the edges.
    ///
    /// Returns an `Err` if adding any of the edges would cause a cycle.
    pub fn from_edges<I>(edges: I) -> Result<Self, WouldCycle<E>>
    where
        I: IntoIterator,
        I::Item: IntoWeightedEdge<E>,
        <I::Item as IntoWeightedEdge<E>>::NodeId: Into<NodeIndex<Ix>>,
        N: Default,
    {
        let mut dag = Self::default();
        dag.extend_with_edges(edges)?;
        Ok(dag)
    }

    /// Extend the `StableDag` with the given edges.
    ///
    /// Node weights `N` are set to default values.
    ///
    /// Edge weights `E` may either be specified in the list, or they are filled with default
    /// values.
    ///
    /// Nodes are inserted automatically to match the edges.
    ///
    /// Returns an `Err` if adding an edge would cause a cycle.
    pub fn extend_with_edges<I>(&mut self, edges: I) -> Result<(), WouldCycle<E>>
    where
        I: IntoIterator,
        I::Item: IntoWeightedEdge<E>,
        <I::Item as IntoWeightedEdge<E>>::NodeId: Into<NodeIndex<Ix>>,
        N: Default,
    {
        for edge in edges {
            let (source, target, weight) = edge.into_weighted_edge();
            let (source, target) = (source.into(), target.into());
            let nx = ::std::cmp::max(source, target);
            while nx.index() >= self.node_count() {
                self.add_node(N::default());
            }
            self.add_edge(source, target, weight)?;
        }
        Ok(())
    }

    /// Create a `StableDag` from an iterator yielding elements.
    ///
    /// Returns an `Err` if an edge would cause a cycle within the graph.
    pub fn from_elements<I>(elements: I) -> Result<Self, WouldCycle<E>>
    where
        Self: Sized,
        I: IntoIterator<Item = pg::data::Element<N, E>>,
    {
        let mut dag = Self::default();
        for elem in elements {
            match elem {
                pg::data::Element::Node { weight } => {
                    dag.add_node(weight);
                }
                pg::data::Element::Edge {
                    source,
                    target,
                    weight,
                } => {
                    let n = NodeIndex::new(source);
                    let e = NodeIndex::new(target);
                    dag.update_edge(n, e, weight)?;
                }
            }
        }
        Ok(dag)
    }

    /// Create a new `Graph` by mapping node and edge weights to new values.
    ///
    /// The resulting graph has the same structure and the same graph indices as `self`.
    pub fn map<'a, F, G, N2, E2>(&'a self, node_map: F, edge_map: G) -> StableDag<N2, E2, Ix>
    where
        F: FnMut(NodeIndex<Ix>, &'a N) -> N2,
        G: FnMut(EdgeIndex<Ix>, &'a E) -> E2,
    {
        let graph = self.graph.map(node_map, edge_map);
        let cycle_state = self.cycle_state.clone();
        StableDag {
            graph: graph,
            cycle_state: cycle_state,
        }
    }

    /// Create a new `StableDag` by mapping node and edge weights. A node or edge may be mapped to
    /// `None` to exclude it from the resulting `StableDag`.
    ///
    /// Nodes are mapped first with the `node_map` closure, then `edge_map` is called for the edges
    /// that have not had any endpoint removed.
    ///
    /// The resulting graph has the structure of a subgraph of the original graph. Nodes and edges
    /// that are not removed maintain their old node or edge indices.
    pub fn filter_map<'a, F, G, N2, E2>(&'a self, node_map: F, edge_map: G) -> StableDag<N2, E2, Ix>
    where
        F: FnMut(NodeIndex<Ix>, &'a N) -> Option<N2>,
        G: FnMut(EdgeIndex<Ix>, &'a E) -> Option<E2>,
    {
        let graph = self.graph.filter_map(node_map, edge_map);
        let cycle_state = DfsSpace::new(&graph);
        StableDag {
            graph: graph,
            cycle_state: cycle_state,
        }
    }

    /// Removes all nodes and edges from the **StableDag**.
    pub fn clear(&mut self) {
        self.graph.clear();
    }

    /// The total number of nodes in the **StableDag**.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// The total number of edges in the **StableDag**.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Borrow the `StableDag`'s underlying `StableDiGraph<N, Ix>`.
    ///
    /// All existing indices may be used to index into this `StableDiGraph` the same way they may be
    /// used to index into the `StableDag`.
    pub fn graph(&self) -> &StableDiGraph<N, E, Ix> {
        &self.graph
    }

    /// Take ownership of the `StableDag` and return the internal `StableDiGraph`.
    ///
    /// All existing indices may be used to index into this `StableDiGraph` the same way they may be
    /// used to index into the `StableDag`.
    pub fn into_graph(self) -> StableDiGraph<N, E, Ix> {
        let StableDag { graph, .. } = self;
        graph
    }

    /// Add a new node to the `StableDag` with the given weight.
    ///
    /// Computes in **O(1)** time.
    ///
    /// Returns the index of the new node.
    ///
    /// **Note:** If you're adding a new node and immediately adding a single edge to that node from
    /// some other node, consider using the [add_child](./struct.StableDag.html#method.add_child) or
    /// [add_parent](./struct.StableDag.html#method.add_parent) methods instead for better
    /// performance.
    ///
    /// **Panics** if the Graph is at the maximum number of nodes for its index type.
    pub fn add_node(&mut self, weight: N) -> NodeIndex<Ix> {
        self.graph.add_node(weight)
    }

    /// Add a new directed edge to the `StableDag` with the given weight.
    ///
    /// The added edge will be in the direction `a` -> `b`
    ///
    /// Checks if the edge would create a cycle in the Graph.
    ///
    /// If adding the edge **would not** cause the graph to cycle, the edge will be added and its
    /// `EdgeIndex` returned.
    ///
    /// If adding the edge **would** cause the graph to cycle, the edge will not be added and
    /// instead a `WouldCycle<E>` error with the given weight will be returned.
    ///
    /// In the worst case, petgraph's [`is_cyclic_directed`][1]
    /// function is used to check whether or not adding the edge would create a cycle.
    ///
    /// **Note:** StableDag allows adding parallel ("duplicate") edges. If you want to avoid this,
    /// use [`update_edge`][2] instead.
    ///
    /// **Note:** If you're adding a new node and immediately adding a single edge to that node from
    /// some other node, consider using the [add_child][3] or
    /// [add_parent][4] methods instead for better
    /// performance.
    ///
    /// **Panics** if either `a` or `b` do not exist within the **StableDag**.
    ///
    /// **Panics** if the Graph is at the maximum number of edges for its index type.
    ///
    ///
    /// [1]: petgraph::algo::is_cyclic_directed
    /// [2]: StableDag::update_edge
    /// [3]: StableDag::add_child
    /// [4]: StableDag::add_parent
    pub fn add_edge(
        &mut self,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
        weight: E,
    ) -> Result<EdgeIndex<Ix>, WouldCycle<E>> {
        let should_check_for_cycle = must_check_for_cycle(self, a, b);
        let state = Some(&mut self.cycle_state);
        if should_check_for_cycle && has_path_connecting(&self.graph, b, a, state) {
            return Err(WouldCycle(weight));
        }

        Ok(self.graph.add_edge(a, b, weight))
    }

    /// Adds the given directed edges to the `StableDag`, each with their own given weight.
    ///
    /// The given iterator should yield a `NodeIndex` pair along with a weight for each Edge to be
    /// added in a tuple.
    ///
    /// If we were to describe the tuple as *(a, b, weight)*, the connection would be directed as
    /// follows:
    ///
    /// *a -> b*
    ///
    /// This method behaves similarly to the [`add_edge`][1]
    /// method, however rather than checking whether or not a cycle has been created after adding
    /// each edge, it only checks after all edges have been added. This makes it a slightly more
    /// performant and ergonomic option that repeatedly calling `add_edge`.
    ///
    /// If adding the edges **would not** cause the graph to cycle, the edges will be added and
    /// their indices returned in an `EdgeIndices` iterator, yielding indices for each edge in the
    /// same order that they were given.
    ///
    /// If adding the edges **would** cause the graph to cycle, the edges will not be added and
    /// instead a `WouldCycle<Vec<E>>` error with the unused weights will be returned. The order of
    /// the returned `Vec` will be the reverse of the given order.
    ///
    /// **Note:** StableDag allows adding parallel ("duplicate") edges. If you want to avoid this,
    /// use [`update_edge`][2] instead.
    ///
    /// **Note:** If you're adding a series of new nodes and edges to a single node, consider using
    ///  the [add_child][3] or [add_parent][4] methods instead for greater convenience.
    ///
    /// **Panics** if the Graph is at the maximum number of nodes for its index type.
    ///
    ///
    /// [1]: StableDag::add_edge
    /// [2]: StableDag::update_edge
    /// [3]: StableDag::add_child
    /// [4]: StableDag::add_parent
    pub fn add_edges<I>(&mut self, edges: I) -> Result<EdgeIndices<Ix>, WouldCycle<Vec<E>>>
    where
        I: IntoIterator<Item = (NodeIndex<Ix>, NodeIndex<Ix>, E)>,
    {
        let mut num_edges = 0;
        let mut should_check_for_cycle = false;

        for (a, b, weight) in edges {
            // Check whether or not we'll need to check for cycles.
            if !should_check_for_cycle {
                if must_check_for_cycle(self, a, b) {
                    should_check_for_cycle = true;
                }
            }

            self.graph.add_edge(a, b, weight);
            num_edges += 1;
        }

        let total_edges = self.edge_count();
        let new_edges_range = total_edges - num_edges..total_edges;

        // Check if adding the edges has created a cycle.
        if should_check_for_cycle && pg::algo::is_cyclic_directed(&self.graph) {
            let removed_edges = new_edges_range.rev().filter_map(|i| {
                let idx = EdgeIndex::new(i);
                self.graph.remove_edge(idx)
            });
            Err(WouldCycle(removed_edges.collect()))
        } else {
            Ok(EdgeIndices {
                indices: new_edges_range,
                _phantom: ::std::marker::PhantomData,
            })
        }
    }

    /// Update the edge from nodes `a` -> `b` with the given weight.
    ///
    /// If the edge doesn't already exist, it will be added using the `add_edge` method.
    ///
    /// Please read the [`add_edge`](./struct.StableDag.html#method.add_edge) for more important
    /// details.
    ///
    /// Checks if the edge would create a cycle in the Graph.
    ///
    /// Computes in **O(t + e)** time where "t" is the complexity of `add_edge` and e is the number
    /// of edges connected to the nodes a and b.
    ///
    /// Returns the index of the edge, or a `WouldCycle` error if adding the edge would create a
    /// cycle.
    ///
    /// **Note:** If you're adding a new node and immediately adding a single edge to that node from
    /// some parent node, consider using the [`add_child`](./struct.StableDag.html#method.add_child)
    /// method instead for greater convenience.
    ///
    /// **Panics** if the Graph is at the maximum number of nodes for its index type.
    pub fn update_edge(
        &mut self,
        a: NodeIndex<Ix>,
        b: NodeIndex<Ix>,
        weight: E,
    ) -> Result<EdgeIndex<Ix>, WouldCycle<E>> {
        if let Some(edge_idx) = self.find_edge(a, b) {
            if let Some(edge) = self.edge_weight_mut(edge_idx) {
                *edge = weight;
                return Ok(edge_idx);
            }
        }
        self.add_edge(a, b, weight)
    }

    /// Find and return the index to the edge that describes `a` -> `b` if there is one.
    ///
    /// Computes in **O(e')** time, where **e'** is the number of edges connected to the nodes `a`
    /// and `b`.
    pub fn find_edge(&self, a: NodeIndex<Ix>, b: NodeIndex<Ix>) -> Option<EdgeIndex<Ix>> {
        self.graph.find_edge(a, b)
    }

    /// Access the parent and child nodes for the given `EdgeIndex`.
    pub fn edge_endpoints(&self, e: EdgeIndex<Ix>) -> Option<(NodeIndex<Ix>, NodeIndex<Ix>)> {
        self.graph.edge_endpoints(e)
    }

    /// Remove all edges.
    pub fn clear_edges(&mut self) {
        self.graph.clear_edges()
    }

    /// Add a new edge and parent node to the node at the given `NodeIndex`.
    ///
    /// Returns both the edge's `EdgeIndex` and the node's `NodeIndex`.
    ///
    /// node -> edge -> child
    ///
    /// Computes in **O(1)** time.
    ///
    /// This is faster than using `add_node` and `add_edge`. This is because we don't have to check
    /// if the graph would cycle when adding an edge to the new node, as we know it it will be the
    /// only edge connected to that node.
    ///
    /// **Panics** if the given child node doesn't exist.
    ///
    /// **Panics** if the Graph is at the maximum number of edges for its index.
    pub fn add_parent(
        &mut self,
        child: NodeIndex<Ix>,
        edge: E,
        node: N,
    ) -> (EdgeIndex<Ix>, NodeIndex<Ix>) {
        let parent_node = self.graph.add_node(node);
        let parent_edge = self.graph.add_edge(parent_node, child, edge);
        (parent_edge, parent_node)
    }

    /// Add a new edge and child node to the node at the given `NodeIndex`.
    /// Returns both the edge's `EdgeIndex` and the node's `NodeIndex`.
    ///
    /// child -> edge -> node
    ///
    /// Computes in **O(1)** time.
    ///
    /// This is faster than using `add_node` and `add_edge`. This is because we don't have to check
    /// if the graph would cycle when adding an edge to the new node, as we know it it will be the
    /// only edge connected to that node.
    ///
    /// **Panics** if the given parent node doesn't exist.
    ///
    /// **Panics** if the Graph is at the maximum number of edges for its index.
    pub fn add_child(
        &mut self,
        parent: NodeIndex<Ix>,
        edge: E,
        node: N,
    ) -> (EdgeIndex<Ix>, NodeIndex<Ix>) {
        let child_node = self.graph.add_node(node);
        let child_edge = self.graph.add_edge(parent, child_node, edge);
        (child_edge, child_node)
    }

    /// Borrow the weight from the node at the given index.
    pub fn node_weight(&self, node: NodeIndex<Ix>) -> Option<&N> {
        self.graph.node_weight(node)
    }

    /// Mutably borrow the weight from the node at the given index.
    pub fn node_weight_mut(&mut self, node: NodeIndex<Ix>) -> Option<&mut N> {
        self.graph.node_weight_mut(node)
    }

    /// Borrow the weight from the edge at the given index.
    pub fn edge_weight(&self, edge: EdgeIndex<Ix>) -> Option<&E> {
        self.graph.edge_weight(edge)
    }

    /// Mutably borrow the weight from the edge at the given index.
    pub fn edge_weight_mut(&mut self, edge: EdgeIndex<Ix>) -> Option<&mut E> {
        self.graph.edge_weight_mut(edge)
    }

    /// Index the `StableDag` by two indices.
    ///
    /// Both indices can be either `NodeIndex`s, `EdgeIndex`s or a combination of the two.
    ///
    /// **Panics** if the indices are equal or if they are out of bounds.
    pub fn index_twice_mut<A, B>(
        &mut self,
        a: A,
        b: B,
    ) -> (
        &mut <StableDiGraph<N, E, Ix> as Index<A>>::Output,
        &mut <StableDiGraph<N, E, Ix> as Index<B>>::Output,
    )
    where
        StableDiGraph<N, E, Ix>: IndexMut<A> + IndexMut<B>,
        A: GraphIndex,
        B: GraphIndex,
    {
        self.graph.index_twice_mut(a, b)
    }

    /// Remove the node at the given index from the `StableDag` and return it if it exists.
    pub fn remove_node(&mut self, node: NodeIndex<Ix>) -> Option<N> {
        self.graph.remove_node(node)
    }

    /// Whether or not the graph contains a node for the given index.
    pub fn contains_node(&self, a: NodeIndex<Ix>) -> bool {
        self.graph.contains_node(a)
    }

    /// Remove an edge and return its weight, or `None` if it didn't exist.
    ///
    /// Computes in **O(e')** time, where **e'** is the size of four particular edge lists, for the
    /// nodes of **e** and the nodes of another affected edge.
    pub fn remove_edge(&mut self, e: EdgeIndex<Ix>) -> Option<E> {
        self.graph.remove_edge(e)
    }

    /// A **Walker** type that may be used to step through the parents of the given child node.
    ///
    /// Unlike iterator types, **Walker**s do not require borrowing the internal **Graph**. This
    /// makes them useful for traversing the **Graph** while still being able to mutably borrow it.
    ///
    /// If you require an iterator, use one of the **Walker** methods for converting this
    /// **Walker** into a similarly behaving **Iterator** type.
    ///
    /// See the [**Walker**](Walker) trait for more useful methods.
    pub fn parents(&self, child: NodeIndex<Ix>) -> Parents<N, E, Ix> {
        let walk_edges = self.graph.neighbors_directed(child, pg::Incoming).detach();
        Parents {
            walk_edges: walk_edges,
            _node: PhantomData,
            _edge: PhantomData,
        }
    }

    /// A "walker" object that may be used to step through the children of the given parent node.
    ///
    /// Unlike iterator types, **Walker**s do not require borrowing the internal **Graph**. This
    /// makes them useful for traversing the **Graph** while still being able to mutably borrow it.
    ///
    /// If you require an iterator, use one of the **Walker** methods for converting this
    /// **Walker** into a similarly behaving **Iterator** type.
    ///
    /// See the [**Walker**](Walker) trait for more useful methods.
    pub fn children(&self, parent: NodeIndex<Ix>) -> Children<N, E, Ix> {
        let walk_edges = self.graph.neighbors_directed(parent, pg::Outgoing).detach();
        Children {
            walk_edges: walk_edges,
            _node: PhantomData,
            _edge: PhantomData,
        }
    }

    /// A **Walker** type that recursively walks the **StableDag** using the given `recursive_fn`.
    ///
    /// See the [**Walker**](Walker) trait for more useful methods.
    pub fn recursive_walk<F>(
        &self,
        start: NodeIndex<Ix>,
        recursive_fn: F,
    ) -> RecursiveWalk<N, E, Ix, F>
    where
        F: FnMut(&Self, NodeIndex<Ix>) -> Option<(EdgeIndex<Ix>, NodeIndex<Ix>)>,
    {
        walker::Recursive::new(start, recursive_fn)
    }
}

/// After adding a new edge to the graph, we use this function immediately after to check whether
/// or not we need to check for a cycle.
///
/// If our parent *a* has no parents or our child *b* has no children, or if there was already an
/// edge connecting *a* to *b*, we know that adding this edge has not caused the graph to cycle.
fn must_check_for_cycle<N, E, Ix>(
    dag: &StableDag<N, E, Ix>,
    a: NodeIndex<Ix>,
    b: NodeIndex<Ix>,
) -> bool
where
    Ix: IndexType,
{
    if a == b {
        return true;
    }
    dag.parents(a).walk_next(dag).is_some()
        && dag.children(b).walk_next(dag).is_some()
        && dag.find_edge(a, b).is_none()
}

// Dag implementations.

impl<N, E, Ix> Into<StableDiGraph<N, E, Ix>> for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn into(self) -> StableDiGraph<N, E, Ix> {
        self.into_graph()
    }
}

impl<N, E, Ix> Default for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn default() -> Self {
        StableDag::new()
    }
}

impl<N, E, Ix> GraphBase for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type NodeId = NodeIndex<Ix>;
    type EdgeId = EdgeIndex<Ix>;
}

impl<N, E, Ix> NodeCount for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn node_count(&self) -> usize {
        StableDag::node_count(self)
    }
}

impl<N, E, Ix> GraphProp for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type EdgeType = pg::Directed;
    fn is_directed(&self) -> bool {
        true
    }
}

impl<N, E, Ix> pg::visit::Visitable for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type Map = <StableDiGraph<N, E, Ix> as Visitable>::Map;
    fn visit_map(&self) -> Self::Map {
        self.graph.visit_map()
    }
    fn reset_map(&self, map: &mut Self::Map) {
        self.graph.reset_map(map)
    }
}

impl<N, E, Ix> pg::visit::Data for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type NodeWeight = N;
    type EdgeWeight = E;
}

impl<N, E, Ix> pg::data::DataMap for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn node_weight(&self, id: Self::NodeId) -> Option<&Self::NodeWeight> {
        self.graph.node_weight(id)
    }
    fn edge_weight(&self, id: Self::EdgeId) -> Option<&Self::EdgeWeight> {
        self.graph.edge_weight(id)
    }
}

impl<N, E, Ix> pg::data::DataMapMut for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn node_weight_mut(&mut self, id: Self::NodeId) -> Option<&mut Self::NodeWeight> {
        self.graph.node_weight_mut(id)
    }
    fn edge_weight_mut(&mut self, id: Self::EdgeId) -> Option<&mut Self::EdgeWeight> {
        self.graph.edge_weight_mut(id)
    }
}

impl<'a, N, E, Ix> IntoNeighbors for &'a StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type Neighbors = pg::stable_graph::Neighbors<'a, E, Ix>;
    fn neighbors(self, n: NodeIndex<Ix>) -> Self::Neighbors {
        self.graph.neighbors(n)
    }
}

impl<'a, N, E, Ix> IntoNeighborsDirected for &'a StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type NeighborsDirected = pg::stable_graph::Neighbors<'a, E, Ix>;
    fn neighbors_directed(self, n: NodeIndex<Ix>, d: pg::Direction) -> Self::Neighbors {
        self.graph.neighbors_directed(n, d)
    }
}

impl<'a, N, E, Ix> IntoEdgeReferences for &'a StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type EdgeRef = pg::stable_graph::EdgeReference<'a, E, Ix>;
    type EdgeReferences = pg::stable_graph::EdgeReferences<'a, E, Ix>;
    fn edge_references(self) -> Self::EdgeReferences {
        self.graph.edge_references()
    }
}

impl<'a, N, E, Ix> IntoEdges for &'a StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type Edges = Edges<'a, E, Ix>;
    fn edges(self, a: Self::NodeId) -> Self::Edges {
        self.graph.edges(a)
    }
}

impl<'a, N, E, Ix> IntoEdgesDirected for &'a StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type EdgesDirected = Edges<'a, E, Ix>;
    fn edges_directed(self, a: Self::NodeId, dir: pg::Direction) -> Self::EdgesDirected {
        self.graph.edges_directed(a, dir)
    }
}

impl<'a, N, E, Ix> IntoNodeIdentifiers for &'a StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type NodeIdentifiers = pg::stable_graph::NodeIndices<'a, N, Ix>;
    fn node_identifiers(self) -> Self::NodeIdentifiers {
        self.graph.node_identifiers()
    }
}

impl<'a, N, E, Ix> IntoNodeReferences for &'a StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type NodeRef = (NodeIndex<Ix>, &'a N);
    type NodeReferences = pg::stable_graph::NodeReferences<'a, N, Ix>;
    fn node_references(self) -> Self::NodeReferences {
        self.graph.node_references()
    }
}

impl<N, E, Ix> NodeIndexable for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn node_bound(&self) -> usize {
        self.graph.node_bound()
    }
    fn to_index(&self, ix: NodeIndex<Ix>) -> usize {
        self.graph.to_index(ix)
    }
    fn from_index(&self, ix: usize) -> Self::NodeId {
        self.graph.from_index(ix)
    }
}

impl<N, E, Ix> NodeCompactIndexable for StableDag<N, E, Ix> where Ix: IndexType {}

impl<N, E, Ix> Index<NodeIndex<Ix>> for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type Output = N;
    fn index(&self, index: NodeIndex<Ix>) -> &N {
        &self.graph[index]
    }
}

impl<N, E, Ix> IndexMut<NodeIndex<Ix>> for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn index_mut(&mut self, index: NodeIndex<Ix>) -> &mut N {
        &mut self.graph[index]
    }
}

impl<N, E, Ix> Index<EdgeIndex<Ix>> for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type Output = E;
    fn index(&self, index: EdgeIndex<Ix>) -> &E {
        &self.graph[index]
    }
}

impl<N, E, Ix> IndexMut<EdgeIndex<Ix>> for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn index_mut(&mut self, index: EdgeIndex<Ix>) -> &mut E {
        &mut self.graph[index]
    }
}

impl<N, E, Ix> GetAdjacencyMatrix for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    type AdjMatrix = <StableDiGraph<N, E, Ix> as GetAdjacencyMatrix>::AdjMatrix;
    fn adjacency_matrix(&self) -> Self::AdjMatrix {
        self.graph.adjacency_matrix()
    }
    fn is_adjacent(&self, matrix: &Self::AdjMatrix, a: NodeIndex<Ix>, b: NodeIndex<Ix>) -> bool {
        self.graph.is_adjacent(matrix, a, b)
    }
}

impl<'a, N, E, Ix> Walker<&'a StableDag<N, E, Ix>> for Children<N, E, Ix>
where
    Ix: IndexType,
{
    type Item = (EdgeIndex<Ix>, NodeIndex<Ix>);
    #[inline]
    fn walk_next(&mut self, dag: &'a StableDag<N, E, Ix>) -> Option<Self::Item> {
        self.walk_edges.next(&dag.graph)
    }
}

impl<'a, N, E, Ix> Walker<&'a StableDag<N, E, Ix>> for Parents<N, E, Ix>
where
    Ix: IndexType,
{
    type Item = (EdgeIndex<Ix>, NodeIndex<Ix>);
    #[inline]
    fn walk_next(&mut self, dag: &'a StableDag<N, E, Ix>) -> Option<Self::Item> {
        self.walk_edges.next(&dag.graph)
    }
}

impl<Ix> Iterator for EdgeIndices<Ix>
where
    Ix: IndexType,
{
    type Item = EdgeIndex<Ix>;
    fn next(&mut self) -> Option<EdgeIndex<Ix>> {
        self.indices.next().map(|i| EdgeIndex::new(i))
    }
}

/// Convert a `Dag` into a `StableDag`
///
/// The resulting graph has the same node and edge indices as
/// the original graph.
impl<N, E, Ix> From<Dag<N, E, Ix>> for StableDag<N, E, Ix>
where
    Ix: IndexType,
{
    fn from(g: Dag<N, E, Ix>) -> Self {
        StableDag {
            graph: StableDiGraph::from(g.graph),
            cycle_state: DfsSpace::default(),
        }
    }
}
