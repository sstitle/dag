//! Directed acyclic graph data structure and utilities.

mod error;
mod graph;
mod ids;
mod policy;

#[cfg(feature = "serde")]
mod json;

pub use error::DagError;
pub use graph::Dag;
pub use ids::{EdgeId, NodeId};
pub use policy::{CheckCycles, CyclePolicy, SkipCycleCheck};

#[cfg(feature = "serde")]
pub use json::{DEFAULT_MAX_DAG_JSON_BYTES, DagJsonError, parse_dag_from_json_str};

#[cfg(test)]
mod tests;
