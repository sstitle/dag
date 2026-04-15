use slotmap::{DefaultKey, Key, KeyData, SlotMap};
use std::collections::{HashSet, VecDeque};
use thiserror::Error;

#[cfg(test)]
mod tests;

// ── ID types ──────────────────────────────────────────────────────────────────

/// Opaque identifier for a node; newtype over u64 (encoded slotmap key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

/// Opaque identifier for an edge; newtype over u64 (encoded slotmap key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub u64);

impl NodeId {
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
}

// ── Internal storage ──────────────────────────────────────────────────────────

struct NodeData<N> {
    meta: N,
    /// Edges leaving this node.
    out_edges: Vec<EdgeId>,
    /// Edges arriving at this node.
    in_edges: Vec<EdgeId>,
}

struct EdgeData<E> {
    from: NodeId,
    to: NodeId,
    meta: E,
}

// ── DAG ───────────────────────────────────────────────────────────────────────

/// A directed acyclic graph generic over node metadata `N` and edge metadata `E`.
pub struct Dag<N, E> {
    nodes: SlotMap<DefaultKey, NodeData<N>>,
    edges: SlotMap<DefaultKey, EdgeData<E>>,
}

impl<N, E> Default for Dag<N, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N, E> Dag<N, E> {
    pub fn new() -> Self {
        Dag {
            nodes: SlotMap::new(),
            edges: SlotMap::new(),
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
    /// Returns [`DagError::CycleDetected`] if the edge would create a cycle.
    pub fn add_edge(&mut self, from: NodeId, to: NodeId, meta: E) -> Result<EdgeId, DagError> {
        if !self.nodes.contains_key(from.key()) {
            return Err(DagError::NodeNotFound(from));
        }
        if !self.nodes.contains_key(to.key()) {
            return Err(DagError::NodeNotFound(to));
        }

        // Self-loop is always a cycle.
        if from == to {
            return Err(DagError::CycleDetected);
        }

        // A cycle exists iff `from` is already reachable from `to`.
        if self.reachable(to, from) {
            return Err(DagError::CycleDetected);
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

    /// Kahn's algorithm — returns a valid topological ordering.
    /// Ties are broken by [`NodeId`] value for determinism.
    pub fn topological_sort(&self) -> Vec<NodeId> {
        // Build in-degree map.
        let mut in_degree: std::collections::HashMap<NodeId, usize> = self
            .nodes
            .iter()
            .map(|(k, n)| (NodeId::from(k), n.in_edges.len()))
            .collect();

        // Seed queue with zero-in-degree nodes, sorted for determinism.
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
