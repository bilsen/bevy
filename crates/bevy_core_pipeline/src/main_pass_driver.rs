use bevy_ecs::world::World;
use bevy_render::{
    camera::{CameraPlugin, ExtractedCameraNames},
    render_graph::{Node, NodeRunError, QueueGraphs, RenderGraphContext, SlotValue},
};

pub struct MainPassDriverNode;

impl Node for MainPassDriverNode {
    fn queue_graphs(
        &self,
        graph: &RenderGraphContext,
        world: &World,
    ) -> Result<QueueGraphs, NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        let mut queued_graphs = QueueGraphs::default();

        if let Some(camera_2d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_2D) {
            queued_graphs.queue(
                graph,
                crate::draw_2d_graph::NAME,
                vec![(
                    crate::draw_2d_graph::input::VIEW_ENTITY,
                    SlotValue::Entity(*camera_2d),
                )],
            )?;
        }

        if let Some(camera_3d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_3D) {
            queued_graphs.queue(
                graph,
                crate::draw_3d_graph::NAME,
                vec![(
                    crate::draw_3d_graph::input::VIEW_ENTITY,
                    SlotValue::Entity(*camera_3d),
                )],
            )?;
        }

        Ok(queued_graphs)
    }
}
