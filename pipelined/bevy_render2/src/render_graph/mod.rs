mod context;
mod edge;
mod graph;
mod graph_slot;
mod node;
mod labels;

pub use context::*;
pub use edge::*;
pub use graph::*;
pub use graph_slot::*;
pub use node::*;
pub use labels::*;

use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    #[error("node does not exist")]
    InvalidNode(NodeLabel),
    #[error("attempted to add an edge that already exists")]
    EdgeAlreadyExists((NodeLabel, NodeLabel)),
    #[error("adding edge would lead to circular dependencies")]
    CircularDependency(Vec<NodeLabel>),
    #[error("slot requested by node is not avaliable")]
    SlotError,
}
