import math
import json
import pytest
from dag import (
    Dag,
    NodeId,
    EdgeId,
    DagNodeNotFoundError,
    DagEdgeNotFoundError,
    DagCycleError,
    DagDuplicateEdgeError,
)


# ── helpers ───────────────────────────────────────────────────────────────────

def make_chain():
    """Build n1 → n2 → n3."""
    dag = Dag()
    n1 = dag.add_node("a")
    n2 = dag.add_node("b")
    n3 = dag.add_node("c")
    dag.add_edge(n1, n2, None)
    dag.add_edge(n2, n3, None)
    return dag, n1, n2, n3


# ── basic construction ────────────────────────────────────────────────────────

def test_add_node_and_meta():
    dag = Dag()
    n = dag.add_node({"key": "value"})
    assert dag.node_meta(n) == {"key": "value"}


def test_add_edge_and_meta():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, 42)
    assert dag.edge_meta(e) == 42


def test_set_node_meta():
    dag = Dag()
    n = dag.add_node(1)
    dag.set_node_meta(n, 2)
    assert dag.node_meta(n) == 2


def test_set_edge_meta():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, "old")
    dag.set_edge_meta(e, "new")
    assert dag.edge_meta(e) == "new"


def test_node_id_equality_and_hash():
    dag = Dag()
    n = dag.add_node(None)
    nodes = dag.nodes()
    assert n in nodes
    assert n == nodes[0]
    assert hash(n) == hash(nodes[0])


def test_edge_id_equality_and_hash():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, None)
    edges = dag.edges()
    assert e in edges
    assert e == edges[0]
    assert hash(e) == hash(edges[0])


# ── NodeId / EdgeId ordering ──────────────────────────────────────────────────

def test_node_id_lt():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    # Nodes are allocated in insertion order; n1 < n2.
    assert n1 < n2
    assert not n2 < n1


def test_node_id_sortable():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    n3 = dag.add_node(None)
    shuffled = [n3, n1, n2]
    assert sorted(shuffled) == [n1, n2, n3]


def test_edge_id_lt():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    n3 = dag.add_node(None)
    e1 = dag.add_edge(n1, n2, None)
    e2 = dag.add_edge(n2, n3, None)
    assert e1 < e2
    assert not e2 < e1


def test_node_id_le_ge():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    assert n1 <= n2
    assert n2 >= n1
    assert n1 <= n1
    assert n1 >= n1


# ── exception types ───────────────────────────────────────────────────────────

def test_node_not_found_raises_correct_exception():
    dag = Dag()
    n = dag.add_node(None)
    dag.remove_node(n)
    with pytest.raises(DagNodeNotFoundError):
        dag.node_meta(n)


def test_node_not_found_is_exception_subclass():
    assert issubclass(DagNodeNotFoundError, Exception)


def test_edge_not_found_raises_correct_exception():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, None)
    dag.remove_edge(e)
    with pytest.raises(DagEdgeNotFoundError):
        dag.edge_meta(e)


def test_edge_not_found_is_exception_subclass():
    assert issubclass(DagEdgeNotFoundError, Exception)


def test_cycle_raises_correct_exception():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    dag.add_edge(n1, n2, None)
    with pytest.raises(DagCycleError):
        dag.add_edge(n2, n1, None)


def test_self_loop_raises_cycle_error():
    dag = Dag()
    n = dag.add_node(None)
    with pytest.raises(DagCycleError):
        dag.add_edge(n, n, None)


def test_cycle_error_is_exception_subclass():
    assert issubclass(DagCycleError, Exception)


# ── duplicate-edge rejection ──────────────────────────────────────────────────

def test_duplicate_edge_raises():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    dag.add_edge(n1, n2, None)
    with pytest.raises(DagDuplicateEdgeError):
        dag.add_edge(n1, n2, None)


def test_duplicate_edge_error_is_exception_subclass():
    assert issubclass(DagDuplicateEdgeError, Exception)


def test_fan_in_not_duplicate():
    """Two different sources → same child is not a duplicate."""
    dag = Dag()
    r1 = dag.add_node(None)
    r2 = dag.add_node(None)
    child = dag.add_node(None)
    dag.add_edge(r1, child, None)
    dag.add_edge(r2, child, None)  # different `from` — must not raise


# ── non-finite float rejection ────────────────────────────────────────────────

def test_nan_metadata_raises():
    dag = Dag()
    with pytest.raises(ValueError):
        dag.add_node(float("nan"))


def test_inf_metadata_raises():
    dag = Dag()
    with pytest.raises(ValueError):
        dag.add_node(float("inf"))


def test_neg_inf_metadata_raises():
    dag = Dag()
    with pytest.raises(ValueError):
        dag.add_node(float("-inf"))


def test_nan_in_nested_list_raises():
    dag = Dag()
    with pytest.raises(ValueError):
        dag.add_node([1, float("nan"), 3])


def test_finite_float_allowed():
    dag = Dag()
    n = dag.add_node(3.14)
    assert dag.node_meta(n) == pytest.approx(3.14)


# ── remove_edge ───────────────────────────────────────────────────────────────

def test_remove_edge_basic():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, None)
    dag.remove_edge(e)
    assert not dag.has_path(n1, n2)


def test_remove_edge_preserves_nodes():
    dag = Dag()
    n1 = dag.add_node("x")
    n2 = dag.add_node("y")
    e = dag.add_edge(n1, n2, None)
    dag.remove_edge(e)
    assert dag.node_meta(n1) == "x"
    assert dag.node_meta(n2) == "y"


def test_remove_edge_nonexistent_raises():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, None)
    dag.remove_edge(e)
    with pytest.raises(DagEdgeNotFoundError):
        dag.remove_edge(e)


def test_remove_edge_cleans_adjacency():
    dag, n1, n2, _ = make_chain()
    e = next(
        e for e in dag.edges() if dag.edge_endpoints(e) == (n1, n2)
    )
    dag.remove_edge(e)
    assert n1 in dag.roots()
    assert n1 in dag.leaves()
    assert n2 in dag.roots()


# ── nodes / edges ─────────────────────────────────────────────────────────────

def test_nodes_empty():
    dag = Dag()
    assert dag.nodes() == []


def test_nodes_returns_all():
    dag, n1, n2, n3 = make_chain()
    nodes = dag.nodes()
    assert len(nodes) == 3
    assert n1 in nodes
    assert n2 in nodes
    assert n3 in nodes


def test_nodes_after_remove():
    dag, n1, n2, n3 = make_chain()
    dag.remove_node(n2)
    nodes = dag.nodes()
    assert n1 in nodes
    assert n2 not in nodes
    assert n3 in nodes


def test_edges_empty():
    dag = Dag()
    assert dag.edges() == []


def test_edges_returns_all():
    dag, *_ = make_chain()
    assert len(dag.edges()) == 2


def test_edges_after_remove_edge():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, None)
    dag.remove_edge(e)
    assert dag.edges() == []


# ── edge_endpoints ────────────────────────────────────────────────────────────

def test_edge_endpoints():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, None)
    from_node, to_node = dag.edge_endpoints(e)
    assert from_node == n1
    assert to_node == n2


def test_edge_endpoints_nonexistent_raises():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, None)
    dag.remove_edge(e)
    with pytest.raises(DagEdgeNotFoundError):
        dag.edge_endpoints(e)


# ── ancestors / descendants ───────────────────────────────────────────────────

def test_ancestors():
    dag, n1, n2, n3 = make_chain()
    anc = dag.ancestors(n3)
    assert len(anc) == 2
    assert n1 in anc
    assert n2 in anc


def test_descendants():
    dag, n1, n2, n3 = make_chain()
    desc = dag.descendants(n1)
    assert len(desc) == 2
    assert n2 in desc
    assert n3 in desc


# ── roots / leaves ────────────────────────────────────────────────────────────

def test_roots_and_leaves():
    dag, n1, _, n3 = make_chain()
    assert n1 in dag.roots()
    assert n3 in dag.leaves()


def test_isolated_node_is_root_and_leaf():
    dag = Dag()
    n = dag.add_node(None)
    assert n in dag.roots()
    assert n in dag.leaves()


# ── topological sort ──────────────────────────────────────────────────────────

def test_topological_sort_order():
    dag, n1, n2, n3 = make_chain()
    order = dag.topological_sort()
    assert order.index(n1) < order.index(n2)
    assert order.index(n2) < order.index(n3)


def test_topological_sort_empty():
    assert Dag().topological_sort() == []


# ── has_path ──────────────────────────────────────────────────────────────────

def test_has_path():
    dag, n1, _, n3 = make_chain()
    assert dag.has_path(n1, n3)
    assert not dag.has_path(n3, n1)


# ── serialization ─────────────────────────────────────────────────────────────

def test_to_json_produces_valid_json():
    dag, *_ = make_chain()
    s = dag.to_json()
    parsed = json.loads(s)
    assert isinstance(parsed, dict)


def test_to_from_json_roundtrip():
    dag, n1, n2, n3 = make_chain()
    dag2 = Dag.from_json(dag.to_json())

    assert dag2.node_meta(n1) == "a"
    assert dag2.node_meta(n2) == "b"
    assert dag2.node_meta(n3) == "c"
    assert dag2.has_path(n1, n3)
    assert not dag2.has_path(n3, n1)


def test_to_from_json_empty():
    dag2 = Dag.from_json(Dag().to_json())
    assert dag2.nodes() == []
    assert dag2.edges() == []


def test_from_json_preserves_edge_endpoints():
    dag = Dag()
    n1 = dag.add_node(None)
    n2 = dag.add_node(None)
    e = dag.add_edge(n1, n2, None)
    dag2 = Dag.from_json(dag.to_json())
    from_node, to_node = dag2.edge_endpoints(e)
    assert from_node == n1
    assert to_node == n2


def test_from_json_invalid_raises():
    with pytest.raises(Exception):
        Dag.from_json("not valid json")


def test_from_json_rejects_oversized_string():
    with pytest.raises(ValueError, match="maximum size"):
        Dag.from_json("x" * 100, max_bytes=10)
