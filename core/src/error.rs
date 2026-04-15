use crate::ids::{EdgeId, NodeId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DagError {
    #[error("node {0:?} not found")]
    NodeNotFound(NodeId),
    #[error("edge {0:?} not found")]
    EdgeNotFound(EdgeId),
    #[error("adding edge would create a cycle")]
    CycleDetected,
    #[error("an edge from {0:?} to {1:?} already exists")]
    DuplicateEdge(NodeId, NodeId),
    #[error("graph contains a cycle; topological ordering cannot include every node")]
    NotAcyclic,
}
