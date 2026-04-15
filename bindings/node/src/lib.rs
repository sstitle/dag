#![deny(clippy::all)]

use dag_core::{
    DEFAULT_MAX_DAG_JSON_BYTES, Dag, DagError, DagJsonError, EdgeId, NodeId,
    parse_dag_from_json_str,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;

/// Maximum integer exactly representable in a JavaScript `number` (IEEE-754 double).
const JS_MAX_SAFE_INTEGER: u64 = (1u64 << 53) - 1;

/// Parse a JS `number` passed in as a node or edge id: must be finite, non-negative,
/// an integer, and within the safe integer range so it round-trips through `f64`.
fn f64_to_u64_id(id: f64, label: &str) -> std::result::Result<u64, napi::Error> {
    if !id.is_finite() {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("DAG_INVALID_ID: {label} must be a finite number"),
        ));
    }
    if id < 0.0 {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("DAG_INVALID_ID: {label} must be non-negative"),
        ));
    }
    if id > JS_MAX_SAFE_INTEGER as f64 {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("DAG_INVALID_ID: {label} exceeds JavaScript safe integer range (2^53-1)"),
        ));
    }
    let u = id as u64;
    if u as f64 != id {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("DAG_INVALID_ID: {label} must be an integer"),
        ));
    }
    Ok(u)
}

fn node_id_from_f64(id: f64, label: &str) -> std::result::Result<NodeId, napi::Error> {
    Ok(NodeId::from_raw(f64_to_u64_id(id, label)?))
}

fn edge_id_from_f64(id: f64, label: &str) -> std::result::Result<EdgeId, napi::Error> {
    Ok(EdgeId::from_raw(f64_to_u64_id(id, label)?))
}

/// Maps [`DagError`] to [`napi::Error`] with stable `DAG_*` message prefixes so
/// callers can distinguish cases without custom exception classes.
fn dag_json_err_to_napi(e: DagJsonError) -> napi::Error {
    match e {
        DagJsonError::TooLarge { len, max } => napi::Error::new(
            napi::Status::InvalidArg,
            format!("DAG_JSON_TOO_LARGE: JSON exceeds maximum size ({len} bytes, max {max} bytes)"),
        ),
        DagJsonError::Serde(e) => {
            napi::Error::new(napi::Status::InvalidArg, format!("DAG_JSON_PARSE: {e}"))
        }
    }
}

fn dag_error_to_napi(e: DagError) -> napi::Error {
    match e {
        DagError::NodeNotFound(_) => {
            napi::Error::new(napi::Status::InvalidArg, format!("DAG_NODE_NOT_FOUND: {e}"))
        }
        DagError::EdgeNotFound(_) => {
            napi::Error::new(napi::Status::InvalidArg, format!("DAG_EDGE_NOT_FOUND: {e}"))
        }
        DagError::CycleDetected | DagError::NotAcyclic => napi::Error::new(
            napi::Status::GenericFailure,
            format!("DAG_CYCLE_DETECTED: {e}"),
        ),
        DagError::DuplicateEdge(_, _) => napi::Error::new(
            napi::Status::GenericFailure,
            format!("DAG_DUPLICATE_EDGE: {e}"),
        ),
    }
}

/// Convert a `NodeId` to a JavaScript `number` (f64).
///
/// JavaScript numbers are IEEE 754 doubles with a 53-bit mantissa, so any
/// integer up to 2^53 − 1 round-trips losslessly.
fn try_node_id_to_f64(id: NodeId) -> std::result::Result<f64, napi::Error> {
    let raw = id.raw();
    if raw > JS_MAX_SAFE_INTEGER {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!(
                "DAG_ID_NOT_REPRESENTABLE: node id {raw} exceeds JavaScript safe integer range (2^53 − 1)"
            ),
        ));
    }
    Ok(raw as f64)
}

/// Convert an `EdgeId` to a JavaScript `number` (f64).
fn try_edge_id_to_f64(id: EdgeId) -> std::result::Result<f64, napi::Error> {
    let raw = id.raw();
    if raw > JS_MAX_SAFE_INTEGER {
        return Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!(
                "DAG_ID_NOT_REPRESENTABLE: edge id {raw} exceeds JavaScript safe integer range (2^53 − 1)"
            ),
        ));
    }
    Ok(raw as f64)
}

/// Stable prefix for `DAG_NODE_NOT_FOUND` errors (use with `String.prototype.startsWith` or
/// `includes`, or compare to [`DAG_ERROR_CODE_NODE_NOT_FOUND`]).
#[napi]
pub const DAG_ERROR_CODE_NODE_NOT_FOUND: &'static str = "DAG_NODE_NOT_FOUND";

#[napi]
pub const DAG_ERROR_CODE_EDGE_NOT_FOUND: &'static str = "DAG_EDGE_NOT_FOUND";

#[napi]
pub const DAG_ERROR_CODE_CYCLE: &'static str = "DAG_CYCLE_DETECTED";

#[napi]
pub const DAG_ERROR_CODE_DUPLICATE_EDGE: &'static str = "DAG_DUPLICATE_EDGE";

#[napi]
pub const DAG_ERROR_CODE_INVALID_ID: &'static str = "DAG_INVALID_ID";

#[napi]
pub const DAG_ERROR_CODE_ID_NOT_REPRESENTABLE: &'static str = "DAG_ID_NOT_REPRESENTABLE";

#[napi]
pub const DAG_ERROR_CODE_JSON_TOO_LARGE: &'static str = "DAG_JSON_TOO_LARGE";

#[napi]
pub const DAG_ERROR_CODE_JSON_PARSE: &'static str = "DAG_JSON_PARSE";

/// Default maximum JSON string length accepted by [`JsDag::from_json`] (256 MiB).
#[napi]
pub fn default_max_dag_json_bytes() -> u32 {
    DEFAULT_MAX_DAG_JSON_BYTES.min(u32::MAX as usize) as u32
}

/// Directed acyclic graph with arbitrary JSON metadata on nodes and edges.
///
/// Node IDs and edge IDs are returned as plain JavaScript `number` values.
/// When passed back into methods, they must be **non-negative integers** within
/// JavaScript’s safe integer range (`Number.MIN_SAFE_INTEGER` …
/// `Number.MAX_SAFE_INTEGER`); non-integers and out-of-range values are rejected.
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
    pub fn add_node(&mut self, meta: Value) -> Result<f64> {
        try_node_id_to_f64(self.inner.add_node(meta))
    }

    /// Remove a node and all its incident edges.
    #[napi]
    pub fn remove_node(&mut self, id: f64) -> Result<()> {
        self.inner
            .remove_node(node_id_from_f64(id, "node id")?)
            .map_err(dag_error_to_napi)
    }

    /// Add a directed edge `from → to` carrying `meta`.
    /// Throws if the edge would create a cycle or if the edge already exists.
    #[napi]
    pub fn add_edge(&mut self, from: f64, to: f64, meta: Value) -> Result<f64> {
        self.inner
            .add_edge(
                node_id_from_f64(from, "from")?,
                node_id_from_f64(to, "to")?,
                meta,
            )
            .map_err(dag_error_to_napi)
            .and_then(try_edge_id_to_f64)
    }

    /// Remove a single edge by ID, leaving its endpoint nodes intact.
    #[napi]
    pub fn remove_edge(&mut self, id: f64) -> Result<()> {
        self.inner
            .remove_edge(edge_id_from_f64(id, "edge id")?)
            .map_err(dag_error_to_napi)
    }

    /// All node IDs currently in the graph (unordered).
    #[napi]
    pub fn nodes(&self) -> Result<Vec<f64>> {
        self.inner.iter_nodes().map(try_node_id_to_f64).collect()
    }

    /// All edge IDs currently in the graph (unordered).
    #[napi]
    pub fn edges(&self) -> Result<Vec<f64>> {
        self.inner.iter_edges().map(try_edge_id_to_f64).collect()
    }

    /// Return the `[from, to]` endpoint node IDs of edge `id`.
    #[napi]
    pub fn edge_endpoints(&self, id: f64) -> Result<Vec<f64>> {
        let (from, to) = self
            .inner
            .edge_endpoints(edge_id_from_f64(id, "edge id")?)
            .map_err(dag_error_to_napi)?;
        Ok(vec![try_node_id_to_f64(from)?, try_node_id_to_f64(to)?])
    }

    /// Return all ancestors of `id` (nodes from which it is reachable).
    ///
    /// **Order is unspecified** — do not rely on BFS/DFS ordering; order may
    /// also differ across processes (hash randomisation).
    #[napi]
    pub fn ancestors(&self, id: f64) -> Result<Vec<f64>> {
        let ids = self
            .inner
            .ancestors(node_id_from_f64(id, "node id")?)
            .map_err(dag_error_to_napi)?;
        ids.into_iter().map(try_node_id_to_f64).collect()
    }

    /// Return all descendants of `id` (nodes reachable from it).
    ///
    /// **Order is unspecified** — do not rely on BFS/DFS ordering; order may
    /// also differ across processes (hash randomisation).
    #[napi]
    pub fn descendants(&self, id: f64) -> Result<Vec<f64>> {
        let ids = self
            .inner
            .descendants(node_id_from_f64(id, "node id")?)
            .map_err(dag_error_to_napi)?;
        ids.into_iter().map(try_node_id_to_f64).collect()
    }

    /// Nodes with no incoming edges.
    #[napi]
    pub fn roots(&self) -> Result<Vec<f64>> {
        self.inner
            .roots()
            .into_iter()
            .map(try_node_id_to_f64)
            .collect()
    }

    /// Nodes with no outgoing edges.
    #[napi]
    pub fn leaves(&self) -> Result<Vec<f64>> {
        self.inner
            .leaves()
            .into_iter()
            .map(try_node_id_to_f64)
            .collect()
    }

    /// A valid topological ordering of all nodes.
    #[napi]
    pub fn topological_sort(&self) -> Result<Vec<f64>> {
        let ids = self.inner.topological_sort().map_err(dag_error_to_napi)?;
        ids.into_iter().map(try_node_id_to_f64).collect()
    }

    /// Verify that the graph is acyclic (same condition as `topological_sort` succeeding).
    #[napi]
    pub fn validate_acyclic(&self) -> Result<()> {
        self.inner.validate_acyclic().map_err(dag_error_to_napi)
    }

    /// Whether there is a directed path from `from` to `to`.
    #[napi]
    pub fn has_path(&self, from: f64, to: f64) -> Result<bool> {
        self.inner
            .has_path(node_id_from_f64(from, "from")?, node_id_from_f64(to, "to")?)
            .map_err(dag_error_to_napi)
    }

    /// Return the metadata of node `id`.
    #[napi]
    pub fn node_meta(&self, id: f64) -> Result<Value> {
        self.inner
            .node_meta(node_id_from_f64(id, "node id")?)
            .map(|v| v.clone())
            .map_err(dag_error_to_napi)
    }

    /// Replace the metadata of node `id`.
    #[napi]
    pub fn set_node_meta(&mut self, id: f64, meta: Value) -> Result<()> {
        self.inner
            .set_node_meta(node_id_from_f64(id, "node id")?, meta)
            .map_err(dag_error_to_napi)
    }

    /// Return the metadata of edge `id`.
    #[napi]
    pub fn edge_meta(&self, id: f64) -> Result<Value> {
        self.inner
            .edge_meta(edge_id_from_f64(id, "edge id")?)
            .map(|v| v.clone())
            .map_err(dag_error_to_napi)
    }

    /// Replace the metadata of edge `id`.
    #[napi]
    pub fn set_edge_meta(&mut self, id: f64, meta: Value) -> Result<()> {
        self.inner
            .set_edge_meta(edge_id_from_f64(id, "edge id")?, meta)
            .map_err(dag_error_to_napi)
    }

    /// Serialize the DAG to a JSON string.
    ///
    /// The format preserves exact node and edge IDs so a round-trip via
    /// `Dag.fromJson` restores the same IDs.
    #[napi]
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(&self.inner).map_err(|e| napi::Error::from_reason(e.to_string()))
    }

    /// Deserialize a DAG from a JSON string produced by `toJson`.
    ///
    /// By default, rejects strings longer than `defaultMaxDagJsonBytes()` before parsing.
    /// Pass optional `maxBytes` to override (for example in tests).
    #[napi(factory)]
    pub fn from_json(s: String, max_bytes: Option<u32>) -> Result<Self> {
        let max = max_bytes
            .map(|u| u as usize)
            .unwrap_or(DEFAULT_MAX_DAG_JSON_BYTES);
        let inner: Dag<Value, Value> =
            parse_dag_from_json_str(&s, max).map_err(dag_json_err_to_napi)?;
        Ok(JsDag { inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_node_id_above_js_safe_integer() {
        let err = try_node_id_to_f64(NodeId::from_raw(JS_MAX_SAFE_INTEGER + 1)).unwrap_err();
        assert!(
            err.to_string().contains("DAG_ID_NOT_REPRESENTABLE"),
            "unexpected error: {err}"
        );
    }
}
