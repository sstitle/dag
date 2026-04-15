use dag_core::{Dag, DagError, EdgeId, NodeId};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFloat, PyList, PyString};
use serde_json::Value;

// ── JSON ↔ Python conversion ──────────────────────────────────────────────────

fn py_to_json<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>) -> PyResult<Value> {
    use pyo3::types::{PyBool, PyInt};

    if obj.is_none() {
        return Ok(Value::Null);
    }
    // Check bool before int — Python's bool is a subclass of int.
    if obj.is_instance_of::<PyBool>() {
        return Ok(Value::Bool(obj.extract::<bool>()?));
    }
    if obj.is_instance_of::<PyInt>() {
        return Ok(Value::Number(obj.extract::<i64>()?.into()));
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
        let items = list
            .iter()
            .map(|item| py_to_json(py, &item))
            .collect::<PyResult<Vec<_>>>()?;
        return Ok(Value::Array(items));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(key, py_to_json(py, &v)?);
        }
        return Ok(Value::Object(map));
    }
    Err(PyValueError::new_err(format!(
        "cannot convert {} to JSON",
        obj.get_type().name()?
    )))
}

fn json_to_py(py: Python<'_>, val: &Value) -> PyResult<PyObject> {
    match val {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok((*b).into_py(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else {
                Ok(n.as_f64().unwrap_or(0.0).into_py(py))
            }
        }
        Value::String(s) => Ok(s.clone().into_py(py)),
        Value::Array(arr) => {
            let items: Vec<PyObject> = arr
                .iter()
                .map(|v| json_to_py(py, v))
                .collect::<PyResult<_>>()?;
            Ok(PyList::new_bound(py, items).into())
        }
        Value::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (k, v) in map {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

// ── Custom exceptions ─────────────────────────────────────────────────────────

pyo3::create_exception!(dag, DagNodeNotFoundError, pyo3::exceptions::PyException);
pyo3::create_exception!(dag, DagEdgeNotFoundError, pyo3::exceptions::PyException);
pyo3::create_exception!(dag, DagCycleError, pyo3::exceptions::PyException);
pyo3::create_exception!(dag, DagDuplicateEdgeError, pyo3::exceptions::PyException);

fn to_py_err(e: DagError) -> PyErr {
    let msg = e.to_string();
    match e {
        DagError::NodeNotFound(_) => DagNodeNotFoundError::new_err(msg),
        DagError::EdgeNotFound(_) => DagEdgeNotFoundError::new_err(msg),
        DagError::CycleDetected => DagCycleError::new_err(msg),
        DagError::DuplicateEdge(_, _) => DagDuplicateEdgeError::new_err(msg),
    }
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
    /// Raises `DagCycleError` if the edge would create a cycle.
    /// Raises `DagDuplicateEdgeError` if an edge between these nodes already exists.
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

    /// All node IDs currently in the graph (unordered).
    pub fn nodes(&self) -> Vec<PyNodeId> {
        self.inner.nodes().into_iter().map(PyNodeId).collect()
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
    pub fn ancestors(&self, node: &PyNodeId) -> PyResult<Vec<PyNodeId>> {
        self.inner
            .ancestors(node.0)
            .map(|ids| ids.into_iter().map(PyNodeId).collect())
            .map_err(to_py_err)
    }

    /// All descendants of `node` (nodes reachable from it).
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
    pub fn topological_sort(&self) -> Vec<PyNodeId> {
        self.inner
            .topological_sort()
            .into_iter()
            .map(PyNodeId)
            .collect()
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
    #[staticmethod]
    pub fn from_json(s: &str) -> PyResult<Self> {
        let inner: Dag<Value, Value> =
            serde_json::from_str(s).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(PyDag { inner })
    }
}

// ── Module ────────────────────────────────────────────────────────────────────

#[pymodule]
fn dag(m: &Bound<'_, PyModule>) -> PyResult<()> {
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
        "DagDuplicateEdgeError",
        m.py().get_type_bound::<DagDuplicateEdgeError>(),
    )?;
    Ok(())
}
