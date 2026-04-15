use crate::error::DagError;
use crate::ids::{EdgeId, NodeId};
use crate::policy::{CheckCycles, CyclePolicy};
use slotmap::{DefaultKey, SlotMap};
use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
/// With the `serde` feature, serialisation round-trips graph structure and IDs.
/// Deserialisation does **not** enforce acyclicity; use [`Dag::validate_acyclic`]
/// after loading untrusted data.
///
/// # Thread safety
///
/// When `N` and `E` are [`Send`], this type is [`Send`]; when they are [`Sync`],
/// it is [`Sync`]. Concurrent mutation is still undefined behaviour from Rust’s
/// perspective — protect the graph with a lock or use single-threaded access.
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

    /// Iterator over all node IDs (unordered). Prefer this over [`Dag::nodes`]
    /// when you only need to scan without allocating a [`Vec`].
    pub fn iter_nodes(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().map(NodeId::from)
    }

    /// Iterator over all edge IDs (unordered). Prefer this over [`Dag::edges`]
    /// when you only need to scan without allocating a [`Vec`].
    pub fn iter_edges(&self) -> impl Iterator<Item = EdgeId> + '_ {
        self.edges.keys().map(EdgeId::from)
    }

    /// All node IDs currently in the graph (unordered).
    ///
    /// Allocates a new [`Vec`] on every call; prefer [`Dag::iter_nodes`] when
    /// streaming IDs is enough.
    pub fn nodes(&self) -> Vec<NodeId> {
        self.iter_nodes().collect()
    }

    /// All edge IDs currently in the graph (unordered).
    ///
    /// Allocates a new [`Vec`] on every call; prefer [`Dag::iter_edges`] when
    /// streaming IDs is enough.
    pub fn edges(&self) -> Vec<EdgeId> {
        self.iter_edges().collect()
    }

    /// The `(from, to)` endpoint nodes of edge `id`.
    pub fn edge_endpoints(&self, id: EdgeId) -> Result<(NodeId, NodeId), DagError> {
        self.edges
            .get(id.key())
            .map(|e| (e.from, e.to))
            .ok_or(DagError::EdgeNotFound(id))
    }

    /// Returns `true` if `id` refers to a node currently in the graph.
    ///
    /// Useful after [`NodeId::from_raw`] (FFI) to check that an ID is still live.
    pub fn has_node(&self, id: NodeId) -> bool {
        self.nodes.contains_key(id.key())
    }

    /// Returns `true` if `id` refers to an edge currently in the graph.
    ///
    /// Useful after [`EdgeId::from_raw`] (FFI) to check that an ID is still live.
    pub fn has_edge(&self, id: EdgeId) -> bool {
        self.edges.contains_key(id.key())
    }

    // ── Graph queries ─────────────────────────────────────────────────────────

    /// All **upstream** nodes of `id`: the transitive predecessors, i.e. every
    /// vertex `u` such that there is a directed path `u → … → id` (equivalently:
    /// nodes **from which** `id` is reachable).
    ///
    /// **Order is unspecified** — do not rely on BFS/DFS ordering. Because a
    /// [`HashSet`] is used internally, iteration order can also differ **across
    /// processes** (hash randomisation), not just between calls in one run.
    /// Building the result [`Vec`] also allocates proportional to the ancestor
    /// count.
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

    /// All **downstream** nodes of `id`: the transitive successors, i.e. every
    /// vertex `v` such that there is a directed path `id → … → v` (nodes
    /// **reachable from** `id`).
    ///
    /// **Order is unspecified** — do not rely on BFS/DFS ordering. As with
    /// [`Dag::ancestors`], order may differ across processes due to hash
    /// randomisation, and the returned [`Vec`] allocates.
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
    /// # Errors
    ///
    /// Returns [`DagError::NotAcyclic`] if the graph contains a cycle (for example
    /// after using [`crate::policy::SkipCycleCheck`], or if a cyclic graph was deserialized from
    /// JSON while using the default [`CheckCycles`] policy for inserts).
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, DagError> {
        let mut order = Vec::with_capacity(self.nodes.len());
        self.kahn_run(|node| order.push(node))?;
        Ok(order)
    }

    /// Verifies that the graph is acyclic (has a topological ordering).
    ///
    /// Equivalent to [`Dag::topological_sort`] succeeding, but makes intent
    /// explicit when validating data loaded from JSON or built with
    /// [`crate::policy::SkipCycleCheck`].
    ///
    /// **Complexity:** **O(V + E)** time and **O(V)** auxiliary space — uses the
    /// same Kahn’s algorithm as [`Dag::topological_sort`] but does not build the
    /// ordering vector. If you need both a pass/fail result **and** the order,
    /// call `topological_sort` once instead of calling this and `topological_sort`
    /// separately (both run the full traversal).
    pub fn validate_acyclic(&self) -> Result<(), DagError> {
        self.kahn_run(|_| ())
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

    /// Kahn’s algorithm. Invokes `consume` once per node in deterministic order
    /// (same tie-breaking as [`Dag::topological_sort`]). Returns
    /// [`DagError::NotAcyclic`] if the graph contains a cycle.
    fn kahn_run(&self, mut consume: impl FnMut(NodeId)) -> Result<(), DagError> {
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

        let mut processed = 0usize;

        while let Some(node) = queue.pop_front() {
            consume(node);
            processed += 1;

            let mut newly_zero: Vec<NodeId> = Vec::new();
            for eid in &self.nodes[node.key()].out_edges {
                let child = self.edges[eid.key()].to;
                let deg = in_degree
                    .get_mut(&child)
                    .expect("in_degree keys mirror the node set; child must exist");
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

        if processed != self.nodes.len() {
            return Err(DagError::NotAcyclic);
        }
        Ok(())
    }

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
