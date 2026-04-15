"""Property-based tests for Python ↔ JSON metadata conversion (integers and floats)."""

from __future__ import annotations

import math
from pathlib import Path

import pytest
from hypothesis import given, strategies as st
from hypothesis import assume

from dag import Dag, DagCycleError


@given(st.integers(min_value=-(2**63), max_value=2**63 - 1))
def test_node_meta_integer_roundtrips_i64_range(n: int) -> None:
    dag = Dag()
    node = dag.add_node(n)
    assert dag.node_meta(node) == n


@given(st.integers(min_value=2**63, max_value=2**64 - 1))
def test_node_meta_integer_roundtrips_u64_exclusive_range(n: int) -> None:
    dag = Dag()
    node = dag.add_node(n)
    assert dag.node_meta(node) == n


def test_integer_above_u64_range_is_rejected() -> None:
    dag = Dag()
    with pytest.raises(ValueError, match="cannot be represented"):
        dag.add_node(2**64)


def test_integer_below_i64_range_is_rejected() -> None:
    dag = Dag()
    with pytest.raises(ValueError, match="cannot be represented"):
        dag.add_node(-(2**63) - 1)


@given(
    st.floats(
        min_value=-1e100,
        max_value=1e100,
        allow_nan=False,
        allow_infinity=False,
        width=64,
    )
)
def test_finite_float_metadata_roundtrip(f: float) -> None:
    assume(not math.isnan(f) and not math.isinf(f))
    dag = Dag()
    node = dag.add_node(f)
    got = dag.node_meta(node)
    assert isinstance(got, float)
    assert got == pytest.approx(f)


def test_cyclic_fixture_deserializes_and_topological_sort_raises() -> None:
    fixture = Path(__file__).parent / "fixtures" / "cyclic_two_node.json"
    dag = Dag.from_json(fixture.read_text())
    assert len(dag.nodes()) == 2
    assert len(dag.edges()) == 2
    with pytest.raises(DagCycleError):
        dag.topological_sort()
