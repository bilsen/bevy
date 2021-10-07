use crate::{
    render_graph::{
        Edge,
        RenderGraphError,
        // RunSubGraphError,
    },
    renderer::RenderContext,
};
use bevy_ecs::{
    system::{In, System},
    world::World,
};
use bevy_utils::Uuid;
use downcast_rs::{impl_downcast, Downcast};
use std::{borrow::Cow, fmt::Debug};
use thiserror::Error;

use super::{GraphContext, RunSubGraph, RunSubGraphs};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NodeId(Uuid);

impl NodeId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        NodeId(Uuid::new_v4())
    }

    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

pub type NodeResult = Result<(RenderContext, RunSubGraphs), NodeRunError>;
pub type NodeInput = (RenderContext, GraphContext);

pub type BoxedNode = Box<dyn System<In = NodeInput, Out = NodeResult>>;


pub trait NodeSystem: System<In = NodeInput, Out = NodeResult> {  }

impl<T: System<In = NodeInput, Out = NodeResult>> NodeSystem for T {  }

#[derive(Error, Debug, Eq, PartialEq)]
pub enum NodeRunError {
    // #[error("encountered an input slot error")]
// InputSlotError(#[from] InputSlotError),
// #[error("encountered an output slot error")]
// OutputSlotError(#[from] OutputSlotError),
// #[error("encountered an error when running a sub-graph")]
// RunSubGraphError(#[from] RunSubGraphError),
}

#[derive(Debug)]
pub struct Edges {
    pub id: NodeId,
    pub input_edges: Vec<Edge>,
    pub output_edges: Vec<Edge>,
}

impl Edges {
    pub(crate) fn add_input_edge(&mut self, edge: Edge) -> Result<(), RenderGraphError> {
        if self.has_input_edge(&edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge));
        }
        self.input_edges.push(edge);
        Ok(())
    }

    pub(crate) fn add_output_edge(&mut self, edge: Edge) -> Result<(), RenderGraphError> {
        if self.has_output_edge(&edge) {
            return Err(RenderGraphError::EdgeAlreadyExists(edge));
        }
        self.output_edges.push(edge);
        Ok(())
    }

    pub fn has_input_edge(&self, edge: &Edge) -> bool {
        self.input_edges.contains(edge)
    }

    pub fn has_output_edge(&self, edge: &Edge) -> bool {
        self.output_edges.contains(edge)
    }
}

pub struct NodeState {
    pub id: NodeId,
    pub name: Option<Cow<'static, str>>,
    pub system: BoxedNode,
    pub edges: Edges,
}

impl Debug for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?} ({:?})", self.id, self.name)
    }
}

impl NodeState {
    pub fn new(id: NodeId, node: BoxedNode) -> Self {
        NodeState {
            id,
            name: Some(node.name()),
            system: node,
            edges: Edges {
                id,
                input_edges: Vec::new(),
                output_edges: Vec::new(),
            },
        }
    }

    pub fn system_mut(&mut self) -> &mut BoxedNode {
        &mut self.system
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NodeLabel {
    Id(NodeId),
    Name(Cow<'static, str>),
}

impl From<&NodeLabel> for NodeLabel {
    fn from(value: &NodeLabel) -> Self {
        value.clone()
    }
}

impl From<String> for NodeLabel {
    fn from(value: String) -> Self {
        NodeLabel::Name(value.into())
    }
}

impl From<&'static str> for NodeLabel {
    fn from(value: &'static str) -> Self {
        NodeLabel::Name(value.into())
    }
}

impl From<NodeId> for NodeLabel {
    fn from(value: NodeId) -> Self {
        NodeLabel::Id(value)
    }
}


pub fn empty_node_system(In((render_context, _graph_context)): In<NodeInput>) -> NodeResult {
    Ok((render_context, Default::default()))
}
