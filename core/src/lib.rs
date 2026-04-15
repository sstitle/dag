use slotmap::{DefaultKey, Key, KeyData, SlotMap};
use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

// ── Sealed trait (prevents external CyclePolicy impls) ───────────────────────

mod private {
    pub trait Sealed {}
}

// ── ID types ──────────────────────────────────────────────────────────────────

/// Opaque identifier for a node; backed by a slotmap key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeId(u64);

/// Opaque identifier for an edge; backed by a slotmap key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeId(u64);

impl NodeId {
    /// Returns the raw `u64` encoding of this ID.
    ///
    /// The encoding is an implementation detail (slotmap FFI key) and may
    /// change across versions. Exposed only for language-binding layers.
    pub fn raw(self) -> u64 {
        self.0
    }

    /// Constructs a `NodeId` from its raw `u64` encoding.
    ///
    /// Intended exclusively for language-binding layers (e.g. the Node.js
    /// binding that round-trips IDs through JavaScript `number`). Using this
    /// to manufacture arbitrary IDs is unsupported and may panic or produce
    /// incorrect results.
    #[doc(hidden)]
    pub fn from_raw(v: u64) -> Self {
        NodeId(v)
    }

    fn key(self) -> DefaultKey {
        DefaultKey::from(KeyData::from_ffi(self.0))
    }
}

impl From<DefaultKey> for NodeId {
    fn from(k: DefaultKey) -> Self {
        NodeId(k.data().as_ffi())
    }
}

impl EdgeId {
    /// Returns the raw `u64` encoding of this ID.
    pub fn raw(self) -> u64 {
        self.0
    }

    /// Constructs an `EdgeId` from its raw `u64` encoding.
    ///
    /// Same caveats as [`NodeId::from_raw`].
    #[doc(hidden)]
    pub fn from_raw(v: u64) -> Self {
        EdgeId(v)
    }

    fn key(self) -> DefaultKey {
        DefaultKey::from(KeyData::from_ffi(self.0))
    }
}

impl From<DefaultKey> for EdgeId {
    fn from(k: DefaultKey) -> Self {
        EdgeId(k.data().as_ffi())
    }
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum DagError {
    #[error("node {0:?} not found")]
    NodeNotFound(NodeId),
    #[error("edge {0:?} not found")]
    EdgeNotFound(EdgeId),
    #[error("adding edge would create a cycle")]
    CycleDetected,
    #[error("an edge from {0:?} to {1:?} already exists")]
    DuplicateEdge(NodeId, NodeId),
}

// ── Cycle policy — dependency-injection hook ──────────────────────────────────

/// Controls whether [`Dag::add_edge`] validates acyclicity.
///
/// Two built-in implementations are provided:
///
/// - [`CheckCycles`] (default) — rejects every edge that would form a cycle.
///   Cost: **O(V + E)** DFS per `add_edge`.
/// - [`SkipCycleCheck`] — skips the reachability scan entirely. Useful when
///   bulk-loading pre-validated data. The caller is responsible for ensuring
///   the resulting graph is acyclic before calling [`Dag::topological_sort`].
///
/// The trait is sealed; only the two types above may implement it.
pub trait CyclePolicy: private::Sealed {
    /// Returns `true` if adding `from → to` would create a cycle.
    ///
    /// `reachable(a, b)` answers whether `b` is reachable from `a` in the
    /// current graph state, without the new edge present.
    fn would_create_cycle(
        reachable: impl Fn(NodeId, NodeId) -> bool,
        from: NodeId,
        to: NodeId,
    ) -> bool;
}

/// Checks acyclicity on every [`Dag::add_edge`] call (O(V + E) DFS).
pub struct CheckCycles;

/// Skips the acyclicity check on [`Dag::add_edge`].
///
/// If you insert edges that create a cycle, [`Dag::topological_sort`] is no
/// longer well-defined: it may return a list shorter than the node count (a
/// partial order) without surfacing an error. **Release builds do not detect
/// this** — only [`debug_assert!`] in [`Dag::topological_sort`] fires in debug
/// builds. This is *not* Rust undefined behaviour; it is a logical error if you
/// treat the result as a full topological order.
pub struct SkipCycleCheck;

impl private::Sealed for CheckCycles {}
impl private::Sealed for SkipCycleCheck {}

impl CyclePolicy for CheckCycles {
    fn would_create_cycle(
        reachable: impl Fn(NodeId, NodeId) -> bool,
        from: NodeId,
        to: NodeId,
    ) -> bool {
        // A cycle arises iff `from` is already reachable from `to`.
        reachable(to, from)
    }
}

impl CyclePolicy for SkipCycleCheck {
    fn would_create_cycle(
        _reachable: impl Fn(NodeId, NodeId) -> bool,
        _from: NodeId,
        _to: NodeId,
    ) -> bool {
        false
    }
}

// ── Internal storage ──────────────────────────────────────────────────────────

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct NodeData<N> {
    meta: N,
    /// Edges leaving this node.
    out_edges: Vec<EdgeId>,
    /// Edges arriving at this node.
    in_edges: Vec<EdgeId>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
struct EdgeData<E> {
    from: NodeId,
    to: NodeId,
    meta: E,
}

// ── DAG ───────────────────────────────────────────────────────────────────────

/// A directed acyclic graph generic over node metadata `N`, edge metadata `E`,
/// and cycle-check policy `P` (defaults to [`CheckCycles`]).
///
/// # Dependency injection
///
/// Supply a custom [`CyclePolicy`] as the third type parameter to swap the
/// cycle-detection strategy at compile time with zero runtime overhead:
///
/// ```rust
/// # use dag_core::{Dag, SkipCycleCheck};
/// let mut dag: Dag<(), (), SkipCycleCheck> = Dag::new();
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "N: serde::Serialize, E: serde::Serialize",
        deserialize = "N: for<'de2> serde::Deserialize<'de2>, \
                       E: for<'de2> serde::Deserialize<'de2>"
    ))
)]
pub struct Dag<N, E, P = CheckCycles> {
    nodes: SlotMap<DefaultKey, NodeData<N>>,
    edges: SlotMap<DefaultKey, EdgeData<E>>,
    /// Zero-size marker; not serialised.
    #[cfg_attr(feature = "serde", serde(skip))]
    _policy: PhantomData<P>,
}

impl<N, E, P: CyclePolicy> Default for Dag<N, E, P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, E, P: CyclePolicy> Dag<N, E, P> {
    pub fn new() -> Self {
        Dag {
            nodes: SlotMap::new(),
            edges: SlotMap::new(),
            _policy: PhantomData,
        }
    }

    // ── Nodes ────────────────────────────────────────────────────────────────

    /// Insert a node with the given metadata; returns its [`NodeId`].
    pub fn add_node(&mut self, meta: N) -> NodeId {
        let key = self.nodes.insert(NodeData {
            meta,
            out_edges: Vec::new(),
            in_edges: Vec::new(),
        });
        NodeId::from(key)
    }

    /// Remove a node and all edges incident to it.
    pub fn remove_node(&mut self, id: NodeId) -> Result<(), DagError> {
        if !self.nodes.contains_key(id.key()) {
            return Err(DagError::NodeNotFound(id));
        }

        // Collect all incident edges before mutating.
        let edges_to_remove: Vec<EdgeId> = {
            let node = &self.nodes[id.key()];
            node.out_edges
                .iter()
                .chain(node.in_edges.iter())
                .copied()
                .collect()
        };

        for eid in edges_to_remove {
            self.remove_edge_raw(eid);
        }

        self.nodes.remove(id.key());
        Ok(())
    }

    /// Return a reference to the metadata of `id`.
    pub fn node_meta(&self, id: NodeId) -> Result<&N, DagError> {
        self.nodes
            .get(id.key())
            .map(|n| &n.meta)
            .ok_or(DagError::NodeNotFound(id))
    }

    /// Replace the metadata of `id`.
    pub fn set_node_meta(&mut self, id: NodeId, meta: N) -> Result<(), DagError> {
        self.nodes
            .get_mut(id.key())
            .map(|n| n.meta = meta)
            .ok_or(DagError::NodeNotFound(id))
    }

    // ── Edges ────────────────────────────────────────────────────────────────

    /// Insert a directed edge `from → to` carrying `meta`.
    ///
    /// **Complexity**: O(V + E) when using the default [`CheckCycles`] policy
    /// (full DFS reachability scan); O(degree(from)) for the duplicate-edge
    /// check regardless of policy.
    ///
    /// # Errors
    ///
    /// - [`DagError::NodeNotFound`] — either endpoint does not exist.
    /// - [`DagError::CycleDetected`] — the edge would form a cycle (always
    ///   returned for self-loops; also returned by [`CheckCycles`] for
    ///   transitive cycles).
    /// - [`DagError::DuplicateEdge`] — an edge between the same `(from, to)`
    ///   pair already exists.
    pub fn add_edge(&mut self, from: NodeId, to: NodeId, meta: E) -> Result<EdgeId, DagError> {
        if !self.nodes.contains_key(from.key()) {
            return Err(DagError::NodeNotFound(from));
        }
        if !self.nodes.contains_key(to.key()) {
            return Err(DagError::NodeNotFound(to));
        }

        // Self-loops are always a cycle, regardless of policy.
        if from == to {
            return Err(DagError::CycleDetected);
        }

        // Policy-injected cycle check (O(V+E) for CheckCycles, O(1) for SkipCycleCheck).
        if P::would_create_cycle(|a, b| self.reachable(a, b), from, to) {
            return Err(DagError::CycleDetected);
        }

        // Reject duplicate (from, to) pairs — O(degree(from)).
        // Checked after the cycle test so that cycle errors take precedence.
        let is_duplicate = self.nodes[from.key()]
            .out_edges
            .iter()
            .any(|&eid| self.edges[eid.key()].to == to);
        if is_duplicate {
            return Err(DagError::DuplicateEdge(from, to));
        }

        let key = self.edges.insert(EdgeData { from, to, meta });
        let eid = EdgeId::from(key);

        self.nodes[from.key()].out_edges.push(eid);
        self.nodes[to.key()].in_edges.push(eid);

        Ok(eid)
    }

    /// Return a reference to the metadata of edge `id`.
    pub fn edge_meta(&self, id: EdgeId) -> Result<&E, DagError> {
        self.edges
            .get(id.key())
            .map(|e| &e.meta)
            .ok_or(DagError::EdgeNotFound(id))
    }

    /// Replace the metadata of edge `id`.
    pub fn set_edge_meta(&mut self, id: EdgeId, meta: E) -> Result<(), DagError> {
        self.edges
            .get_mut(id.key())
            .map(|e| e.meta = meta)
            .ok_or(DagError::EdgeNotFound(id))
    }

    /// Remove a single edge by ID, cleaning up both endpoint adjacency lists.
    pub fn remove_edge(&mut self, id: EdgeId) -> Result<(), DagError> {
        if !self.edges.contains_key(id.key()) {
            return Err(DagError::EdgeNotFound(id));
        }
        self.remove_edge_raw(id);
        Ok(())
    }

    // ── Iteration ─────────────────────────────────────────────────────────────

    /// All node IDs currently in the graph (unordered).
    pub fn nodes(&self) -> Vec<NodeId> {
        self.nodes.keys().map(NodeId::from).collect()
    }

    /// All edge IDs currently in the graph (unordered).
    pub fn edges(&self) -> Vec<EdgeId> {
        self.edges.keys().map(EdgeId::from).collect()
    }

    /// The `(from, to)` endpoint nodes of edge `id`.
    pub fn edge_endpoints(&self, id: EdgeId) -> Result<(NodeId, NodeId), DagError> {
        self.edges
            .get(id.key())
            .map(|e| (e.from, e.to))
            .ok_or(DagError::EdgeNotFound(id))
    }

    // ── Graph queries ─────────────────────────────────────────────────────────

    /// All ancestors of `id` (nodes from which `id` is reachable).
    pub fn ancestors(&self, id: NodeId) -> Result<Vec<NodeId>, DagError> {
        if !self.nodes.contains_key(id.key()) {
            return Err(DagError::NodeNotFound(id));
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([id]);

        while let Some(node) = queue.pop_front() {
            for eid in &self.nodes[node.key()].in_edges {
                let parent = self.edges[eid.key()].from;
                if visited.insert(parent) {
                    queue.push_back(parent);
                }
            }
        }

        Ok(visited.into_iter().collect())
    }

    /// All descendants of `id` (nodes reachable from `id`).
    pub fn descendants(&self, id: NodeId) -> Result<Vec<NodeId>, DagError> {
        if !self.nodes.contains_key(id.key()) {
            return Err(DagError::NodeNotFound(id));
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([id]);

        while let Some(node) = queue.pop_front() {
            for eid in &self.nodes[node.key()].out_edges {
                let child = self.edges[eid.key()].to;
                if visited.insert(child) {
                    queue.push_back(child);
                }
            }
        }

        Ok(visited.into_iter().collect())
    }

    /// Nodes with no incoming edges.
    pub fn roots(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, n)| n.in_edges.is_empty())
            .map(|(k, _)| NodeId::from(k))
            .collect()
    }

    /// Nodes with no outgoing edges.
    pub fn leaves(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, n)| n.out_edges.is_empty())
            .map(|(k, _)| NodeId::from(k))
            .collect()
    }

    /// Kahn's algorithm — returns a valid topological ordering when the graph
    /// is acyclic. Ties are broken by [`NodeId`] value for determinism.
    ///
    /// # Cycles and [`SkipCycleCheck`]
    ///
    /// If the graph contains a cycle (only possible after using
    /// [`SkipCycleCheck`] to bypass cycle checks), this method returns a vector
    /// whose length is **strictly less than** `nodes().len()` — not a valid
    /// topological order of every node. **Release builds do not panic or return
    /// `Err`;** only a [`debug_assert!`] runs in debug builds.
    ///
    /// # Panics (debug builds only)
    ///
    /// [`debug_assert!`] if the graph contains a cycle in the [`SkipCycleCheck`]
    /// case described above.
    pub fn topological_sort(&self) -> Vec<NodeId> {
        let mut in_degree: std::collections::HashMap<NodeId, usize> = self
            .nodes
            .iter()
            .map(|(k, n)| (NodeId::from(k), n.in_edges.len()))
            .collect();

        let mut queue: VecDeque<NodeId> = {
            let mut v: Vec<NodeId> = in_degree
                .iter()
                .filter(|(_, &d)| d == 0)
                .map(|(&id, _)| id)
                .collect();
            v.sort_unstable();
            v.into()
        };

        let mut result = Vec::with_capacity(self.nodes.len());

        while let Some(node) = queue.pop_front() {
            result.push(node);

            let mut newly_zero: Vec<NodeId> = Vec::new();
            for eid in &self.nodes[node.key()].out_edges {
                let child = self.edges[eid.key()].to;
                let deg = in_degree.get_mut(&child).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    newly_zero.push(child);
                }
            }
            newly_zero.sort_unstable();
            for n in newly_zero {
                queue.push_back(n);
            }
        }

        debug_assert_eq!(
            result.len(),
            self.nodes.len(),
            "topological_sort produced a partial result — the graph contains a cycle; \
             this is only possible when using SkipCycleCheck policy"
        );

        result
    }

    /// Returns `true` if there is a directed path from `from` to `to`.
    pub fn has_path(&self, from: NodeId, to: NodeId) -> Result<bool, DagError> {
        if !self.nodes.contains_key(from.key()) {
            return Err(DagError::NodeNotFound(from));
        }
        if !self.nodes.contains_key(to.key()) {
            return Err(DagError::NodeNotFound(to));
        }
        Ok(self.reachable(from, to))
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// DFS reachability check (does not validate that nodes exist).
    fn reachable(&self, from: NodeId, to: NodeId) -> bool {
        if from == to {
            return true;
        }
        let mut visited = HashSet::new();
        let mut stack = vec![from];

        while let Some(node) = stack.pop() {
            if node == to {
                return true;
            }
            if visited.insert(node) {
                for eid in &self.nodes[node.key()].out_edges {
                    stack.push(self.edges[eid.key()].to);
                }
            }
        }
        false
    }

    /// Remove an edge from the slotmap and clean up both endpoints' adjacency lists.
    fn remove_edge_raw(&mut self, eid: EdgeId) {
        if let Some(edge) = self.edges.remove(eid.key()) {
            if let Some(from_node) = self.nodes.get_mut(edge.from.key()) {
                from_node.out_edges.retain(|e| *e != eid);
            }
            if let Some(to_node) = self.nodes.get_mut(edge.to.key()) {
                to_node.in_edges.retain(|e| *e != eid);
            }
        }
    }
}
