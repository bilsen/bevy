use bevy_ecs::world::World;
use bevy_render::render_graph::{Node, NodeRunError, RenderGraphContext, QueueGraphs, SlotValues};

pub struct ClearPassDriverNode;

impl Node for ClearPassDriverNode {
    fn queue_graphs(
        &self,
        graph: &RenderGraphContext,
        _world: &World,
    ) -> Result<QueueGraphs, NodeRunError> {
        let mut queued_graphs = QueueGraphs::default();
        queued_graphs.queue(graph, &crate::clear_graph::NAME, SlotValues::empty())?;

        Ok(queued_graphs)
    }
}
