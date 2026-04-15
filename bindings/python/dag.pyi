"""
Type stubs for the dag Python extension.

Metadata on nodes and edges must be JSON-serialisable (None, bool, int, float,
str, list, or dict with string keys).  Non-finite floats (NaN, ±Inf) are not
valid JSON and will raise ``ValueError``.
"""

from typing import Any, Tuple

# ── Exceptions ────────────────────────────────────────────────────────────────

class DagNodeNotFoundError(Exception):
    """Raised when a NodeId does not exist in the graph."""

class DagEdgeNotFoundError(Exception):
    """Raised when an EdgeId does not exist in the graph."""

class DagCycleError(Exception):
    """Raised when adding an edge would create a cycle."""

class DagDuplicateEdgeError(Exception):
    """Raised when an edge between the given (from, to) pair already exists."""

# ── ID types ──────────────────────────────────────────────────────────────────

class NodeId:
    """Opaque identifier for a node.

    Supports equality, hashing, and full ordering (``<``, ``<=``, ``>``,
    ``>=``), so instances can be sorted and used in sorted containers.
    """

    @property
    def value(self) -> int: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __lt__(self, other: "NodeId") -> bool: ...
    def __le__(self, other: "NodeId") -> bool: ...
    def __gt__(self, other: "NodeId") -> bool: ...
    def __ge__(self, other: "NodeId") -> bool: ...

class EdgeId:
    """Opaque identifier for an edge.

    Supports equality, hashing, and full ordering (``<``, ``<=``, ``>``,
    ``>=``), so instances can be sorted and used in sorted containers.
    """

    @property
    def value(self) -> int: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __lt__(self, other: "EdgeId") -> bool: ...
    def __le__(self, other: "EdgeId") -> bool: ...
    def __gt__(self, other: "EdgeId") -> bool: ...
    def __ge__(self, other: "EdgeId") -> bool: ...

# ── Dag ───────────────────────────────────────────────────────────────────────

class Dag:
    """
    A directed acyclic graph with arbitrary JSON-serialisable metadata on nodes
    and edges.

    All mutating operations take ``self`` by reference; the object is not
    thread-safe without external locking.
    """

    def __init__(self) -> None: ...

    # Nodes

    def add_node(self, meta: Any) -> NodeId:
        """Insert a node with the given metadata and return its NodeId.

        Raises ``ValueError`` if *meta* contains a non-finite float.
        """

    def remove_node(self, node: NodeId) -> None:
        """Remove a node and all edges incident to it.

        Raises DagNodeNotFoundError if the node does not exist.
        """

    def node_meta(self, node: NodeId) -> Any:
        """Return the metadata of *node*.

        Raises DagNodeNotFoundError if the node does not exist.
        """

    def set_node_meta(self, node: NodeId, meta: Any) -> None:
        """Replace the metadata of *node*.

        Raises DagNodeNotFoundError if the node does not exist.
        Raises ``ValueError`` if *meta* contains a non-finite float.
        """

    def nodes(self) -> list[NodeId]:
        """Return all node IDs currently in the graph (unordered)."""

    # Edges

    def add_edge(self, from_node: NodeId, to_node: NodeId, meta: Any) -> EdgeId:
        """Insert a directed edge from_node → to_node carrying *meta*.

        Raises DagCycleError if the edge would create a cycle.
        Raises DagNodeNotFoundError if either endpoint does not exist.
        Raises DagDuplicateEdgeError if an edge between these nodes already exists.
        Raises ``ValueError`` if *meta* contains a non-finite float.
        """

    def remove_edge(self, edge: EdgeId) -> None:
        """Remove a single edge by ID, leaving its endpoint nodes intact.

        Raises DagEdgeNotFoundError if the edge does not exist.
        """

    def edge_meta(self, edge: EdgeId) -> Any:
        """Return the metadata of *edge*.

        Raises DagEdgeNotFoundError if the edge does not exist.
        """

    def set_edge_meta(self, edge: EdgeId, meta: Any) -> None:
        """Replace the metadata of *edge*.

        Raises DagEdgeNotFoundError if the edge does not exist.
        Raises ``ValueError`` if *meta* contains a non-finite float.
        """

    def edge_endpoints(self, edge: EdgeId) -> Tuple[NodeId, NodeId]:
        """Return the ``(from, to)`` endpoint nodes of *edge*.

        Raises DagEdgeNotFoundError if the edge does not exist.
        """

    def edges(self) -> list[EdgeId]:
        """Return all edge IDs currently in the graph (unordered)."""

    # Queries

    def ancestors(self, node: NodeId) -> list[NodeId]:
        """Return all ancestors of *node* (nodes from which it is reachable).

        The returned list is unordered. Raises DagNodeNotFoundError if the node
        does not exist.
        """

    def descendants(self, node: NodeId) -> list[NodeId]:
        """Return all descendants of *node* (nodes reachable from it).

        The returned list is unordered. Raises DagNodeNotFoundError if the node
        does not exist.
        """

    def roots(self) -> list[NodeId]:
        """Return nodes with no incoming edges (unordered)."""

    def leaves(self) -> list[NodeId]:
        """Return nodes with no outgoing edges (unordered)."""

    def topological_sort(self) -> list[NodeId]:
        """Return a valid topological ordering of all nodes.

        Ties are broken deterministically by NodeId value.

        Raises :class:`DagCycleError` if the graph is not acyclic (including a graph
        produced by deserialisation that contains a cycle).
        """

    def has_path(self, from_node: NodeId, to_node: NodeId) -> bool:
        """Return True if there is a directed path from *from_node* to *to_node*.

        Raises DagNodeNotFoundError if either node does not exist.
        """

    # Serialisation

    def to_json(self) -> str:
        """Serialise the DAG to a JSON string.

        The format preserves exact NodeId and EdgeId values so that a
        round-trip via :meth:`from_json` restores the same IDs.
        """

    @staticmethod
    def from_json(s: str) -> "Dag":
        """Deserialise a DAG from a JSON string produced by :meth:`to_json`.

        Raises ValueError if *s* is not valid JSON or does not match the
        expected schema.
        """
