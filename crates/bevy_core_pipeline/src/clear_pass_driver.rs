use bevy_ecs::world::World;
use bevy_render::render_graph::{Node, NodeRunError, QueueContext, QueueNode, SlotValues};

pub struct ClearPassDriverNode;

impl Node for ClearPassDriverNode {}

impl QueueNode for ClearPassDriverNode {
    fn queue(
        &self,
        _slot_values: &SlotValues,
        queue_context: &mut QueueContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        queue_context.queue(crate::clear_graph::NAME, SlotValues::default())?;
        Ok(())
    }
}
