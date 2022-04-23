use bevy_ecs::world::World;
use bevy_render::{
    camera::{ActiveCamera, Camera2d, Camera3d},
    render_graph::{Node, NodeRunError, QueueContext, QueueNode, SlotValues},
};

use crate::{draw_2d_graph, draw_3d_graph};

pub struct MainPassDriverNode;

impl Node for MainPassDriverNode {}

impl QueueNode for MainPassDriverNode {
    fn queue(
        &self,
        _slot_values: &SlotValues,
        queue_context: &mut QueueContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if let Some(camera_3d) = world.resource::<ActiveCamera<Camera3d>>().get() {
            queue_context.queue(
                crate::draw_3d_graph::NAME,
                SlotValues::default().with(draw_3d_graph::input::VIEW_ENTITY, camera_3d),
            )?;
        }

        if let Some(camera_2d) = world.resource::<ActiveCamera<Camera2d>>().get() {
            queue_context.queue(
                crate::draw_2d_graph::NAME,
                SlotValues::default().with(draw_2d_graph::input::VIEW_ENTITY, camera_2d),
            )?;
        }

        Ok(())
    }
}
// impl Node for MainPassDriverNode {
//     fn run(
//         &self,
//         graph: &mut RenderGraphContext,
//         _render_context: &mut RenderContext,
//         world: &World,
//     ) -> Result<(), NodeRunError> {
//         if let Some(camera_3d) = world.resource::<ActiveCamera<Camera3d>>().get() {
//             graph.run_sub_graph(
//                 crate::draw_3d_graph::NAME,
//                 vec![SlotValue::Entity(camera_3d)],
//             )?;
//         }

//         if let Some(camera_2d) = world.resource::<ActiveCamera<Camera2d>>().get() {
//             graph.run_sub_graph(
//                 crate::draw_2d_graph::NAME,
//                 vec![SlotValue::Entity(camera_2d)],
//             )?;
//         }

//         Ok(())
//     }
// }
