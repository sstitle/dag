#[cfg(feature = "serde")]
use crate::graph::Dag;
#[cfg(feature = "serde")]
use crate::policy::CyclePolicy;

/// Default maximum JSON string length accepted by [`parse_dag_from_json_str`]
/// (256 MiB). Intended as a guardrail when deserialising untrusted input.
#[cfg(feature = "serde")]
pub const DEFAULT_MAX_DAG_JSON_BYTES: usize = 256 * 1024 * 1024;

/// Failure when deserialising a [`Dag`] from JSON.
#[cfg(feature = "serde")]
#[derive(Debug, thiserror::Error)]
pub enum DagJsonError {
    #[error("JSON exceeds maximum size ({len} bytes, max {max} bytes)")]
    TooLarge { len: usize, max: usize },
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

/// Deserialise a [`Dag`] from a JSON string, rejecting inputs larger than
/// `max_bytes` before parsing.
///
/// **Integrity:** JSON deserialisation does **not** verify that the graph is
/// acyclic. A malicious or buggy payload can therefore deserialize into a
/// [`Dag`] that contains cycles. For untrusted input, call
/// [`Dag::validate_acyclic`] (or [`Dag::topological_sort`]) after parsing.
///
/// ```ignore
/// let dag: Dag<MyNode, MyEdge> = parse_dag_from_json_str(s, max_bytes)?;
/// dag.validate_acyclic()?;
/// ```
#[cfg(feature = "serde")]
pub fn parse_dag_from_json_str<N, E, P>(
    s: &str,
    max_bytes: usize,
) -> Result<Dag<N, E, P>, DagJsonError>
where
    N: for<'de> serde::Deserialize<'de>,
    E: for<'de> serde::Deserialize<'de>,
    P: CyclePolicy,
{
    if s.len() > max_bytes {
        return Err(DagJsonError::TooLarge {
            len: s.len(),
            max: max_bytes,
        });
    }
    serde_json::from_str(s).map_err(DagJsonError::from)
}
