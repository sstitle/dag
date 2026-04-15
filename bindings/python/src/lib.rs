use dag_core::{
    DEFAULT_MAX_DAG_JSON_BYTES, Dag, DagError, DagJsonError, EdgeId, NodeId,
    parse_dag_from_json_str,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyString};
use serde_json::Value;

/// Maximum nesting depth when converting Python metadata to JSON (lists/dicts)
/// and JSON back to Python. Prevents stack exhaustion on pathological input.
pub const MAX_JSON_CONVERSION_DEPTH: usize = 64;

// ── JSON ↔ Python conversion ──────────────────────────────────────────────────

/// Maps a Python `int` to a `serde_json::Number` (i64, u64, or non-integer f64 via JSON only).
fn python_int_to_json_number(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Number> {
    let v: i128 = obj.extract().map_err(|_| {
        PyValueError::new_err(
            "integer is too large for JSON metadata (must fit in i128 for conversion)",
        )
    })?;
    if let Ok(i) = i64::try_from(v) {
        return Ok(i.into());
    }
    if v >= 0 {
        if let Ok(u) = u64::try_from(v) {
            return Ok(u.into());
        }
    }
    Err(PyValueError::new_err(format!(
        "integer {v} cannot be represented as JSON metadata (supported range is i64 or u64)"
    )))
}

fn py_to_json<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>) -> PyResult<Value> {
    py_to_json_inner(py, obj, MAX_JSON_CONVERSION_DEPTH)
}

fn py_to_json_inner<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    depth: usize,
) -> PyResult<Value> {
    use pyo3::types::{PyBool, PyInt};

    if obj.is_none() {
        return Ok(Value::Null);
    }
    // Check bool before int — Python's bool is a subclass of int.
    if obj.is_instance_of::<PyBool>() {
        return Ok(Value::Bool(obj.extract::<bool>()?));
    }
    if obj.is_instance_of::<PyInt>() {
        return Ok(Value::Number(python_int_to_json_number(obj)?));
    }
    if obj.is_instance_of::<PyFloat>() {
        let f = obj.extract::<f64>()?;
        // JSON does not support NaN or infinite floats — surface the error
        // explicitly rather than silently coercing to null.
        return serde_json::Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "cannot convert non-finite float ({f}) to JSON; \
                     use None or a finite value instead"
                ))
            });
    }
    if obj.is_instance_of::<PyString>() {
        return Ok(Value::String(obj.extract::<String>()?));
    }
    if let Ok(list) = obj.downcast::<PyList>() {
        if depth == 0 {
            return Err(PyValueError::new_err(format!(
                "maximum JSON nesting depth ({}) exceeded when converting Python object",
                MAX_JSON_CONVERSION_DEPTH
            )));
        }
        let items = list
            .iter()
            .map(|item| py_to_json_inner(py, &item, depth - 1))
            .collect::<PyResult<Vec<_>>>()?;
        return Ok(Value::Array(items));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        if depth == 0 {
            return Err(PyValueError::new_err(format!(
                "maximum JSON nesting depth ({}) exceeded when converting Python object",
                MAX_JSON_CONVERSION_DEPTH
            )));
        }
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(key, py_to_json_inner(py, &v, depth - 1)?);
        }
        return Ok(Value::Object(map));
    }
    Err(PyValueError::new_err(format!(
        "cannot convert {} to JSON",
        obj.get_type().name()?
    )))
}

fn json_to_py(py: Python<'_>, val: &Value) -> PyResult<PyObject> {
    json_to_py_inner(py, val, MAX_JSON_CONVERSION_DEPTH)
}

fn json_to_py_inner(py: Python<'_>, val: &Value, depth: usize) -> PyResult<PyObject> {
    match val {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok((*b).into_py(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Err(PyValueError::new_err("invalid JSON number"))
            }
        }
        Value::String(s) => Ok(s.clone().into_py(py)),
        Value::Array(arr) => {
            if depth == 0 {
                return Err(PyValueError::new_err(format!(
                    "maximum JSON nesting depth ({}) exceeded when converting JSON to Python",
                    MAX_JSON_CONVERSION_DEPTH
                )));
            }
            let items: Vec<PyObject> = arr
                .iter()
                .map(|v| json_to_py_inner(py, v, depth - 1))
                .collect::<PyResult<_>>()?;
            Ok(PyList::new_bound(py, items).into())
        }
        Value::Object(map) => {
            if depth == 0 {
                return Err(PyValueError::new_err(format!(
                    "maximum JSON nesting depth ({}) exceeded when converting JSON to Python",
                    MAX_JSON_CONVERSION_DEPTH
                )));
            }
            let dict = PyDict::new_bound(py);
            for (k, v) in map {
                dict.set_item(k, json_to_py_inner(py, v, depth - 1)?)?;
            }
            Ok(dict.into())
        }
    }
}

// ── Custom exceptions ─────────────────────────────────────────────────────────

pyo3::create_exception!(dag, DagNodeNotFoundError, pyo3::exceptions::PyException);
pyo3::create_exception!(dag, DagEdgeNotFoundError, pyo3::exceptions::PyException);
pyo3::create_exception!(dag, DagCycleError, pyo3::exceptions::PyException);
pyo3::create_exception!(dag, DagNotAcyclicError, pyo3::exceptions::PyException);
pyo3::create_exception!(dag, DagDuplicateEdgeError, pyo3::exceptions::PyException);

fn to_py_err(e: DagError) -> PyErr {
    let msg = e.to_string();
    match e {
        DagError::NodeNotFound(_) => DagNodeNotFoundError::new_err(msg),
        DagError::EdgeNotFound(_) => DagEdgeNotFoundError::new_err(msg),
        DagError::CycleDetected => DagCycleError::new_err(msg),
        DagError::NotAcyclic => DagNotAcyclicError::new_err(msg),
        DagError::DuplicateEdge(_, _) => DagDuplicateEdgeError::new_err(msg),
    }
}

fn json_err_to_py(e: DagJsonError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

// ── Python-visible ID types ───────────────────────────────────────────────────

/// Stores the Rust `NodeId` directly to avoid raw-u64 round-trips.
#[pyclass(name = "NodeId")]
#[derive(Clone, Copy)]
pub struct PyNodeId(NodeId);

#[pymethods]
impl PyNodeId {
    fn __repr__(&self) -> String {
        format!("NodeId({})", self.0.raw())
    }
    /// Full ordering support: enables `sorted()`, `bisect`, `<`, `<=`, `>`, `>=`.
    fn __richcmp__(&self, other: &PyNodeId, op: pyo3::basic::CompareOp) -> bool {
        op.matches(self.0.cmp(&other.0))
    }
    fn __hash__(&self) -> u64 {
        self.0.raw()
    }
    #[getter]
    fn value(&self) -> u64 {
        self.0.raw()
    }
}

/// Stores the Rust `EdgeId` directly.
#[pyclass(name = "EdgeId")]
#[derive(Clone, Copy)]
pub struct PyEdgeId(EdgeId);

#[pymethods]
impl PyEdgeId {
    fn __repr__(&self) -> String {
        format!("EdgeId({})", self.0.raw())
    }
    /// Full ordering support: enables `sorted()`, `bisect`, `<`, `<=`, `>`, `>=`.
    fn __richcmp__(&self, other: &PyEdgeId, op: pyo3::basic::CompareOp) -> bool {
        op.matches(self.0.cmp(&other.0))
    }
    fn __hash__(&self) -> u64 {
        self.0.raw()
    }
    #[getter]
    fn value(&self) -> u64 {
        self.0.raw()
    }
}

// ── Dag binding ───────────────────────────────────────────────────────────────

#[pyclass(name = "Dag")]
pub struct PyDag {
    inner: Dag<Value, Value>,
}

#[pymethods]
impl PyDag {
    #[new]
    pub fn new() -> Self {
        PyDag { inner: Dag::new() }
    }

    /// Add a node; `meta` may be any JSON-serialisable Python object.
    pub fn add_node(&mut self, py: Python<'_>, meta: PyObject) -> PyResult<PyNodeId> {
        let json_meta = py_to_json(py, &meta.bind(py).clone())?;
        Ok(PyNodeId(self.inner.add_node(json_meta)))
    }

    /// Remove `node` and all its incident edges.
    pub fn remove_node(&mut self, node: &PyNodeId) -> PyResult<()> {
        self.inner.remove_node(node.0).map_err(to_py_err)
    }

    /// Add a directed edge `from_node → to_node`.
    ///
    /// Raises [`DagCycleError`] if the edge would create a cycle.
    /// Raises [`DagDuplicateEdgeError`] if an edge between these nodes already exists.
    pub fn add_edge(
        &mut self,
        py: Python<'_>,
        from_node: &PyNodeId,
        to_node: &PyNodeId,
        meta: PyObject,
    ) -> PyResult<PyEdgeId> {
        let json_meta = py_to_json(py, &meta.bind(py).clone())?;
        self.inner
            .add_edge(from_node.0, to_node.0, json_meta)
            .map(PyEdgeId)
            .map_err(to_py_err)
    }

    /// Remove a single edge by ID, leaving its endpoint nodes intact.
    pub fn remove_edge(&mut self, edge: &PyEdgeId) -> PyResult<()> {
        self.inner.remove_edge(edge.0).map_err(to_py_err)
    }

    /// Returns `true` if `node` is currently in the graph.
    pub fn has_node(&self, node: &PyNodeId) -> bool {
        self.inner.has_node(node.0)
    }

    /// All node IDs currently in the graph (unordered).
    pub fn nodes(&self) -> Vec<PyNodeId> {
        self.inner.nodes().into_iter().map(PyNodeId).collect()
    }

    /// Returns `true` if `edge` is currently in the graph.
    pub fn has_edge(&self, edge: &PyEdgeId) -> bool {
        self.inner.has_edge(edge.0)
    }

    /// All edge IDs currently in the graph (unordered).
    pub fn edges(&self) -> Vec<PyEdgeId> {
        self.inner.edges().into_iter().map(PyEdgeId).collect()
    }

    /// The `(from, to)` endpoint nodes of an edge.
    pub fn edge_endpoints(&self, edge: &PyEdgeId) -> PyResult<(PyNodeId, PyNodeId)> {
        self.inner
            .edge_endpoints(edge.0)
            .map(|(f, t)| (PyNodeId(f), PyNodeId(t)))
            .map_err(to_py_err)
    }

    /// All ancestors of `node` (nodes from which it is reachable).
    ///
    /// The returned list is **unordered**; do not rely on BFS/DFS ordering.
    /// Order may also differ across processes (hash randomisation).
    pub fn ancestors(&self, node: &PyNodeId) -> PyResult<Vec<PyNodeId>> {
        self.inner
            .ancestors(node.0)
            .map(|ids| ids.into_iter().map(PyNodeId).collect())
            .map_err(to_py_err)
    }

    /// All descendants of `node` (nodes reachable from it).
    ///
    /// The returned list is **unordered**; do not rely on BFS/DFS ordering.
    /// Order may also differ across processes (hash randomisation).
    pub fn descendants(&self, node: &PyNodeId) -> PyResult<Vec<PyNodeId>> {
        self.inner
            .descendants(node.0)
            .map(|ids| ids.into_iter().map(PyNodeId).collect())
            .map_err(to_py_err)
    }

    /// Nodes with no incoming edges.
    pub fn roots(&self) -> Vec<PyNodeId> {
        self.inner.roots().into_iter().map(PyNodeId).collect()
    }

    /// Nodes with no outgoing edges.
    pub fn leaves(&self) -> Vec<PyNodeId> {
        self.inner.leaves().into_iter().map(PyNodeId).collect()
    }

    /// A valid topological ordering of all nodes.
    ///
    /// Raises [`DagNotAcyclicError`] if the graph contains a cycle.
    pub fn topological_sort(&self) -> PyResult<Vec<PyNodeId>> {
        self.inner
            .topological_sort()
            .map(|ids| ids.into_iter().map(PyNodeId).collect())
            .map_err(to_py_err)
    }

    /// Verify that the graph is acyclic (same condition as `topological_sort` succeeding).
    ///
    /// Raises [`DagNotAcyclicError`] if the graph contains a cycle.
    pub fn validate_acyclic(&self) -> PyResult<()> {
        self.inner.validate_acyclic().map_err(to_py_err)
    }

    /// Whether there is a directed path from `from_node` to `to_node`.
    pub fn has_path(&self, from_node: &PyNodeId, to_node: &PyNodeId) -> PyResult<bool> {
        self.inner
            .has_path(from_node.0, to_node.0)
            .map_err(to_py_err)
    }

    /// Return the metadata of `node`.
    pub fn node_meta(&self, py: Python<'_>, node: &PyNodeId) -> PyResult<PyObject> {
        let val = self.inner.node_meta(node.0).map_err(to_py_err)?;
        json_to_py(py, val)
    }

    /// Replace the metadata of `node`.
    pub fn set_node_meta(
        &mut self,
        py: Python<'_>,
        node: &PyNodeId,
        meta: PyObject,
    ) -> PyResult<()> {
        let json_meta = py_to_json(py, &meta.bind(py).clone())?;
        self.inner
            .set_node_meta(node.0, json_meta)
            .map_err(to_py_err)
    }

    /// Return the metadata of `edge`.
    pub fn edge_meta(&self, py: Python<'_>, edge: &PyEdgeId) -> PyResult<PyObject> {
        let val = self.inner.edge_meta(edge.0).map_err(to_py_err)?;
        json_to_py(py, val)
    }

    /// Replace the metadata of `edge`.
    pub fn set_edge_meta(
        &mut self,
        py: Python<'_>,
        edge: &PyEdgeId,
        meta: PyObject,
    ) -> PyResult<()> {
        let json_meta = py_to_json(py, &meta.bind(py).clone())?;
        self.inner
            .set_edge_meta(edge.0, json_meta)
            .map_err(to_py_err)
    }

    /// Serialize the DAG to a JSON string.
    pub fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Deserialize a DAG from a JSON string.
    ///
    /// By default, rejects inputs longer than the module constant
    /// `DEFAULT_MAX_DAG_JSON_BYTES` before parsing. Pass *max_bytes* to override
    /// (for example in tests).
    #[staticmethod]
    #[pyo3(signature = (s, max_bytes=None))]
    pub fn from_json(s: &str, max_bytes: Option<usize>) -> PyResult<Self> {
        let max = max_bytes.unwrap_or(DEFAULT_MAX_DAG_JSON_BYTES);
        let inner: Dag<Value, Value> = parse_dag_from_json_str(s, max).map_err(json_err_to_py)?;
        Ok(PyDag { inner })
    }
}

// ── Module ────────────────────────────────────────────────────────────────────

#[pymodule]
fn dag(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("DEFAULT_MAX_DAG_JSON_BYTES", DEFAULT_MAX_DAG_JSON_BYTES)?;
    m.add("MAX_JSON_CONVERSION_DEPTH", MAX_JSON_CONVERSION_DEPTH)?;
    m.add_class::<PyDag>()?;
    m.add_class::<PyNodeId>()?;
    m.add_class::<PyEdgeId>()?;
    m.add(
        "DagNodeNotFoundError",
        m.py().get_type_bound::<DagNodeNotFoundError>(),
    )?;
    m.add(
        "DagEdgeNotFoundError",
        m.py().get_type_bound::<DagEdgeNotFoundError>(),
    )?;
    m.add("DagCycleError", m.py().get_type_bound::<DagCycleError>())?;
    m.add(
        "DagNotAcyclicError",
        m.py().get_type_bound::<DagNotAcyclicError>(),
    )?;
    m.add(
        "DagDuplicateEdgeError",
        m.py().get_type_bound::<DagDuplicateEdgeError>(),
    )?;
    Ok(())
}
