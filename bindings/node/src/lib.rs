#![deny(clippy::all)]

use dag_core::{Dag, DagError, EdgeId, NodeId};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;

fn to_napi(e: DagError) -> napi::Error {
    napi::Error::from_reason(e.to_string())
}

/// Directed acyclic graph with arbitrary JSON metadata on nodes and edges.
///
/// Node IDs and edge IDs are returned as plain JavaScript `number` values.
/// They are safe to use as numbers for any practical graph size (values stay
/// well below 2^53).
#[napi(js_name = "Dag")]
pub struct JsDag {
    inner: Dag<Value, Value>,
}

#[napi]
impl JsDag {
    /// Create an empty DAG.
    #[napi(constructor)]
    pub fn new() -> Self {
        JsDag { inner: Dag::new() }
    }

    /// Add a node carrying `meta` (any JSON value).
    /// Returns the numeric node ID.
    #[napi]
    pub fn add_node(&mut self, meta: Value) -> f64 {
        self.inner.add_node(meta).0 as f64
    }

    /// Remove a node and all its incident edges.
    #[napi]
    pub fn remove_node(&mut self, id: f64) -> Result<()> {
        self.inner
            .remove_node(NodeId(id as u64))
            .map_err(to_napi)
    }

    /// Add a directed edge `from → to` carrying `meta`.
    /// Throws if the edge would create a cycle.
    #[napi]
    pub fn add_edge(&mut self, from: f64, to: f64, meta: Value) -> Result<f64> {
        self.inner
            .add_edge(NodeId(from as u64), NodeId(to as u64), meta)
            .map(|e| e.0 as f64)
            .map_err(to_napi)
    }

    /// Return all ancestors of `id` (nodes from which it is reachable).
    #[napi]
    pub fn ancestors(&self, id: f64) -> Result<Vec<f64>> {
        self.inner
            .ancestors(NodeId(id as u64))
            .map(|ids| ids.into_iter().map(|n| n.0 as f64).collect())
            .map_err(to_napi)
    }

    /// Return all descendants of `id` (nodes reachable from it).
    #[napi]
    pub fn descendants(&self, id: f64) -> Result<Vec<f64>> {
        self.inner
            .descendants(NodeId(id as u64))
            .map(|ids| ids.into_iter().map(|n| n.0 as f64).collect())
            .map_err(to_napi)
    }

    /// Nodes with no incoming edges.
    #[napi]
    pub fn roots(&self) -> Vec<f64> {
        self.inner.roots().into_iter().map(|n| n.0 as f64).collect()
    }

    /// Nodes with no outgoing edges.
    #[napi]
    pub fn leaves(&self) -> Vec<f64> {
        self.inner.leaves().into_iter().map(|n| n.0 as f64).collect()
    }

    /// A valid topological ordering of all nodes.
    #[napi]
    pub fn topological_sort(&self) -> Vec<f64> {
        self.inner
            .topological_sort()
            .into_iter()
            .map(|n| n.0 as f64)
            .collect()
    }

    /// Whether there is a directed path from `from` to `to`.
    #[napi]
    pub fn has_path(&self, from: f64, to: f64) -> Result<bool> {
        self.inner
            .has_path(NodeId(from as u64), NodeId(to as u64))
            .map_err(to_napi)
    }

    /// Return the metadata of node `id`.
    #[napi]
    pub fn node_meta(&self, id: f64) -> Result<Value> {
        self.inner
            .node_meta(NodeId(id as u64))
            .map(|v| v.clone())
            .map_err(to_napi)
    }

    /// Replace the metadata of node `id`.
    #[napi]
    pub fn set_node_meta(&mut self, id: f64, meta: Value) -> Result<()> {
        self.inner
            .set_node_meta(NodeId(id as u64), meta)
            .map_err(to_napi)
    }

    /// Return the metadata of edge `id`.
    #[napi]
    pub fn edge_meta(&self, id: f64) -> Result<Value> {
        self.inner
            .edge_meta(EdgeId(id as u64))
            .map(|v| v.clone())
            .map_err(to_napi)
    }

    /// Replace the metadata of edge `id`.
    #[napi]
    pub fn set_edge_meta(&mut self, id: f64, meta: Value) -> Result<()> {
        self.inner
            .set_edge_meta(EdgeId(id as u64), meta)
            .map_err(to_napi)
    }
}
