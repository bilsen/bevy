use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::World;
use bevy_reflect::Uuid;
use bevy_utils::define_label;

use crate::renderer::RenderContext;

use super::{
    BoxedRenderGraphLabel, NodeRunError, QueueingError, RenderGraphLabel, RenderGraphs,
    SlotRequirements, SlotValues,
};
pub use bevy_render_macros::RenderNodeLabel;
define_label!(RenderNodeLabel);

pub type BoxedRenderNodeLabel = Box<dyn RenderNodeLabel>;

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

pub trait Node: Send + Sync + 'static {
    fn slot_requirements(&self) -> SlotRequirements {
        SlotRequirements::default()
    }

    fn update(&mut self, _world: &mut World) {}
}

pub trait RecordingNode: Node {
    fn record(
        &self,
        slot_values: &SlotValues,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError>;
}

pub trait QueueNode: Node {
    fn queue(
        &self,
        slot_values: &SlotValues,
        queue_context: &mut QueueContext,
        world: &World,
    ) -> Result<(), NodeRunError>;
}

pub struct NodeState {
    function: BoxedNode,
    id: NodeId,
    label: BoxedRenderNodeLabel,
}

impl NodeState {
    pub fn get_id(&self) -> &NodeId {
        &self.id
    }

    pub fn from_recording(node: impl RecordingNode, label: impl RenderNodeLabel) -> Self {
        let boxed_label = Box::new(label);

        Self {
            function: BoxedNode::Record(Box::new(node)),
            label: boxed_label,
            id: NodeId::new(),
        }
    }

    pub fn from_queue(node: impl QueueNode, label: impl RenderNodeLabel) -> Self {
        let boxed_label = Box::new(label);

        Self {
            function: BoxedNode::Queue(Box::new(node)),
            label: boxed_label,
            id: NodeId::new(),
        }
    }

    pub fn from_empty(node: impl Node, label: impl RenderNodeLabel) -> Self {
        let boxed_label = Box::new(label);

        Self {
            function: BoxedNode::Empty(Box::new(node)),
            label: boxed_label,
            id: NodeId::new(),
        }
    }

    pub fn get_label(&self) -> &BoxedRenderNodeLabel {
        &self.label
    }

    pub fn get_function(&self) -> &BoxedNode {
        &self.function
    }

    pub fn get_function_mut(&mut self) -> &mut BoxedNode {
        &mut self.function
    }

    pub fn requirements(&self) -> SlotRequirements {
        self.function.slot_requirements()
    }

}

pub enum BoxedNode {
    Queue(Box<dyn QueueNode>),
    Record(Box<dyn RecordingNode>),
    Empty(Box<dyn Node>),
}

impl Node for BoxedNode {
    fn slot_requirements(&self) -> SlotRequirements {
        match self {
            BoxedNode::Queue(node) => node.slot_requirements(),
            BoxedNode::Record(node) => node.slot_requirements(),
            BoxedNode::Empty(node) => node.slot_requirements(),
        }
    }

    fn update(&mut self, world: &mut World) {
        match self {
            BoxedNode::Queue(node) => node.update(world),
            BoxedNode::Record(node) => node.update(world),
            BoxedNode::Empty(node) => node.update(world),
        }
    }
}

pub struct QueueContext<'w> {
    graphs: &'w RenderGraphs,
    queue: GraphQueue,
}

impl<'w> QueueContext<'w> {
    pub fn new(graphs: &'w RenderGraphs) -> Self {
        Self {
            graphs,
            queue: GraphQueue::default(),
        }
    }
    pub fn queue(
        &mut self,
        label: impl RenderGraphLabel,
        values: SlotValues,
    ) -> Result<(), QueueingError> {
        let boxed_label = Box::new(label);

        if let Ok(graph) = self.graphs.get(&*boxed_label) {
            if let Some(value_error) = graph.requirements().get_slot_value_error(&values) {
                return Err(QueueingError::SlotValueError {
                    render_graph: format!("{:?}", boxed_label).into(),
                    value_error,
                });
            }
            self.queue.push((boxed_label, values));
            Ok(())
        } else {
            Err(QueueingError::LabelError(
                format!("{:?}", boxed_label).into(),
            ))
        }
    }

    pub fn finish(self) -> GraphQueue {
        self.queue
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct GraphQueue(Vec<(BoxedRenderGraphLabel, SlotValues)>);
