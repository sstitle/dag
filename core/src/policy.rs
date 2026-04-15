use crate::ids::NodeId;

mod private {
    pub trait Sealed {}
}

/// Controls whether [`crate::graph::Dag::add_edge`] validates acyclicity.
///
/// Two built-in implementations are provided:
///
/// - [`CheckCycles`] (default) — rejects every edge that would form a cycle.
///   Cost: **O(V + E)** DFS per `add_edge`.
/// - [`SkipCycleCheck`] — skips the reachability scan entirely. Useful when
///   bulk-loading pre-validated data. The caller is responsible for ensuring
///   the resulting graph is acyclic before calling [`crate::graph::Dag::topological_sort`].
///
/// The trait is sealed; only the two types above may implement it.
pub trait CyclePolicy: private::Sealed {
    /// Returns `true` if adding `from → to` would create a cycle.
    ///
    /// `reachable(a, b)` answers whether `b` is reachable from `a` in the
    /// current graph state, without the new edge present.
    fn would_create_cycle(
        reachable: impl Fn(NodeId, NodeId) -> bool,
        from: NodeId,
        to: NodeId,
    ) -> bool;
}

/// Checks acyclicity on every [`crate::graph::Dag::add_edge`] call (O(V + E) DFS).
pub struct CheckCycles;

/// Skips the acyclicity check on [`crate::graph::Dag::add_edge`].
///
/// If you insert edges that create a cycle, [`crate::graph::Dag::topological_sort`] returns
/// [`crate::DagError::NotAcyclic`].
pub struct SkipCycleCheck;

impl private::Sealed for CheckCycles {}
impl private::Sealed for SkipCycleCheck {}

impl CyclePolicy for CheckCycles {
    fn would_create_cycle(
        reachable: impl Fn(NodeId, NodeId) -> bool,
        from: NodeId,
        to: NodeId,
    ) -> bool {
        // A cycle arises iff `from` is already reachable from `to`.
        reachable(to, from)
    }
}

impl CyclePolicy for SkipCycleCheck {
    fn would_create_cycle(
        _reachable: impl Fn(NodeId, NodeId) -> bool,
        _from: NodeId,
        _to: NodeId,
    ) -> bool {
        false
    }
}
