use crate::{
    render_graph::{RenderGraphContext, RenderGraphError, QueueGraphError, SlotError, SlotInfos},
    renderer::RenderContext,
};
use bevy_ecs::world::World;
use bevy_utils::Uuid;
use downcast_rs::{impl_downcast, Downcast};
use std::{borrow::Cow, fmt::Debug};
use thiserror::Error;

use super::QueueGraphs;


/// A [`Node`] identifier.
/// It automatically generates its own random uuid.
///
/// This id is used to reference the node internally (edges, etc).
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

/// A render node that can be added to a [`RenderGraph`](super::RenderGraph).
///
/// Nodes are the fundamental part of the graph and used to extend its functionality, by
/// generating draw calls and/or running subgraphs.
/// They are added via the `render_graph::add_node(my_node)` method.
///
/// To determine their position in the graph and ensure that all required dependencies (inputs)
/// are already executed, [`Edges`](Edge) are used.
///
/// A node can produce outputs used as dependencies by other nodes.
/// Those inputs and outputs are called slots and are the default way of passing render data
/// inside the graph. For more information see [`SlotType`](super::SlotType).
pub trait Node: Downcast + Send + Sync + 'static {
    /// Specifies the required input slots for this node.
    /// They will then be available during the run method inside the [`RenderContext`].
    fn slot_requirements(&self) -> SlotInfos {
        SlotInfos::default()
    }

    /// Updates internal node state using the current render [`World`] prior to the run method.
    fn update(&mut self, _world: &mut World) {}

    /// Runs the graph node logic, issues draw calls. The graph data is
    /// passed via the [`RenderGraphContext`].
    fn record(
        &self,
        _graph: &RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }

    /// Queues graphs for execution.
    fn queue_graphs(
        &self,
        _graph: &RenderGraphContext,
        _world: &World,
    ) -> Result<QueueGraphs, NodeRunError> {
        Ok(Default::default())
    }
}

impl_downcast!(Node);

#[derive(Error, Debug, Eq, PartialEq)]
pub enum NodeRunError {
    #[error("encountered an slot error")]
    SlotError(#[from] SlotError),
    #[error("encountered an error when queueing a graph")]
    QueueGraphError(#[from] QueueGraphError),
}

/// The internal representation of a [`Node`], with all data required
/// by the [`RenderGraph`](super::RenderGraph).
pub struct NodeState {
    pub id: NodeId,
    pub name: Cow<'static, str>,
    /// The name of the type that implements [`Node`].
    pub type_name: &'static str,
    pub node: Box<dyn Node>,
    pub required_slots: SlotInfos,
    pub dependencies: Vec<NodeId>,
    pub dependants: Vec<NodeId>,
}

impl Debug for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?} ({:?})", self.id, self.name)
    }
}

impl NodeState {
    /// Creates an [`NodeState`] without edges, but the `input_slots` and `output_slots`
    /// are provided by the `node`.
    pub fn new<T>(id: NodeId, node: T) -> Self
    where
        T: Node,
    {
        NodeState {
            id,
            name: "".into(),
            required_slots: node.slot_requirements(),
            node: Box::new(node),
            type_name: std::any::type_name::<T>(),
            dependencies: Vec::new(),
            dependants: Vec::new(),
        }
    }

    /// Retrieves the [`Node`].
    pub fn node<T>(&self) -> Result<&T, RenderGraphError>
    where
        T: Node,
    {
        self.node
            .downcast_ref::<T>()
            .ok_or(RenderGraphError::WrongNodeType)
    }

    /// Retrieves the [`Node`] mutably.
    pub fn node_mut<T>(&mut self) -> Result<&mut T, RenderGraphError>
    where
        T: Node,
    {
        self.node
            .downcast_mut::<T>()
            .ok_or(RenderGraphError::WrongNodeType)
    }

    pub fn iter_dependants(&self) -> impl Iterator<Item = &NodeId> {
        self.dependants.iter()
    }

    pub fn iter_dependencies(&self) -> impl Iterator<Item = &NodeId> {
        self.dependencies.iter()
    }

    pub fn add_dependency(&mut self, id: NodeId) {
        self.dependencies.push(id);
    }

    pub fn add_dependant(&mut self, id: NodeId) {
        self.dependants.push(id);
    }
}

/// A [`NodeLabel`] is used to reference a [`NodeState`] by either its name or [`NodeId`]
/// inside the [`RenderGraph`](super::RenderGraph).
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

/// A [`Node`] without any inputs, outputs and subgraphs, which does nothing when run.
/// Used (as a label) to bundle multiple dependencies into one inside
/// the [`RenderGraph`](super::RenderGraph).
pub struct EmptyNode;

impl Node for EmptyNode {}
