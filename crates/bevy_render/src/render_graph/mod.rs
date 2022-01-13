mod context;
mod graph;
mod node;
mod node_slot;

pub use context::*;
pub use graph::*;
pub use node::*;
pub use node_slot::*;

use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    #[error("node does not exist")]
    InvalidNode(NodeLabel),
    #[error("node does not match the given type")]
    WrongNodeType,
    #[error("attempted to add an edge that already exists")]
    EdgeAlreadyExists(NodeId, NodeId),
}
