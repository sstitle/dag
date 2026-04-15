"""Property tests for JSON round-trip and acyclicity checks (requires ``hypothesis``)."""

from hypothesis import given, strategies as st

from dag import Dag, DagDuplicateEdgeError, DagNotAcyclicError


@given(st.integers(min_value=0, max_value=15))
def test_chain_json_roundtrip_preserves_size_and_acyclicity(n_nodes):
    """A simple chain serialises and deserialises with the same node count and stays acyclic."""
    dag = Dag()
    ids = [dag.add_node({"i": i}) for i in range(n_nodes)]
    for i in range(max(0, n_nodes - 1)):
        dag.add_edge(ids[i], ids[i + 1], None)

    dag2 = Dag.from_json(dag.to_json())
    dag2.validate_acyclic()
    assert len(dag2.nodes()) == n_nodes


@given(st.integers(min_value=0, max_value=12))
def test_validate_acyclic_agrees_with_topological_sort(n_nodes):
    """``validate_acyclic`` and ``topological_sort`` must succeed or fail together."""
    dag = Dag()
    ids = [dag.add_node(None) for _ in range(n_nodes)]
    for i in range(n_nodes - 1):
        dag.add_edge(ids[i], ids[i + 1], None)

    def ok(thunk):
        try:
            thunk()
            return True
        except DagNotAcyclicError:
            return False

    assert ok(lambda: dag.validate_acyclic()) == ok(lambda: dag.topological_sort())


@given(st.integers(min_value=2, max_value=8), st.data())
def test_random_forward_edges_in_random_order_stay_acyclic(n_nodes, data):
    """Build a DAG by only adding edges that go forward in a random permutation (a random topo order)."""
    perm = data.draw(st.permutations(range(n_nodes)))
    dag = Dag()
    ids = [dag.add_node(i) for i in range(n_nodes)]
    pos = {perm[r]: r for r in range(n_nodes)}
    for i in range(n_nodes):
        for j in range(n_nodes):
            if i != j and pos[i] < pos[j]:
                try:
                    dag.add_edge(ids[i], ids[j], None)
                except DagDuplicateEdgeError:
                    pass
    dag.validate_acyclic()
    assert len(dag.topological_sort()) == n_nodes
