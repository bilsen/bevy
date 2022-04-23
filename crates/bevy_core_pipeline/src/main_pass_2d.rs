use crate::{draw_2d_graph, Transparent2d};
use bevy_ecs::prelude::*;
use bevy_render::{
    render_graph::{Node, NodeRunError, RecordingNode, SlotRequirements, SlotValues},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{LoadOp, Operations, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewTarget},
};

pub struct MainPass2dNode {
    query:
        QueryState<(&'static RenderPhase<Transparent2d>, &'static ViewTarget), With<ExtractedView>>,
}

impl MainPass2dNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for MainPass2dNode {
    fn slot_requirements(&self) -> SlotRequirements {
        SlotRequirements::default().with::<Entity>(draw_2d_graph::input::VIEW_ENTITY.into())
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }
}

impl RecordingNode for MainPass2dNode {
    fn record(
        &self,
        slot_values: &SlotValues,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = *slot_values.get::<Entity>(&draw_2d_graph::input::VIEW_ENTITY.into())?;
        let (transparent_phase, target) = self
            .query
            .get_manual(world, view_entity)
            .expect("view entity should exist");

        let pass_descriptor = RenderPassDescriptor {
            label: Some("main_pass_2d"),
            color_attachments: &[target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            })],
            depth_stencil_attachment: None,
        };

        let draw_functions = world.resource::<DrawFunctions<Transparent2d>>();

        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        let mut draw_functions = draw_functions.write();
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        for item in &transparent_phase.items {
            let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
            draw_function.draw(world, &mut tracked_pass, view_entity, item);
        }
        Ok(())
    }
}

// impl Node for MainPass2dNode {
//     fn input(&self) -> Vec<SlotInfo> {
//         vec![SlotInfo::new(MainPass2dNode::IN_VIEW, SlotType::Entity)]
//     }

//     fn update(&mut self, world: &mut World) {
//         self.query.update_archetypes(world);
//     }

//     fn run(
//         &self,
//         graph: &mut RenderGraphContext,
//         render_context: &mut RenderContext,
//         world: &World,
//     ) -> Result<(), NodeRunError> {
//         let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
//         let (transparent_phase, target) = self
//             .query
//             .get_manual(world, view_entity)
//             .expect("view entity should exist");

//         let pass_descriptor = RenderPassDescriptor {
//             label: Some("main_pass_2d"),
//             color_attachments: &[target.get_color_attachment(Operations {
//                 load: LoadOp::Load,
//                 store: true,
//             })],
//             depth_stencil_attachment: None,
//         };

//         let draw_functions = world.resource::<DrawFunctions<Transparent2d>>();

//         let render_pass = render_context
//             .command_encoder
//             .begin_render_pass(&pass_descriptor);

//         let mut draw_functions = draw_functions.write();
//         let mut tracked_pass = TrackedRenderPass::new(render_pass);
//         for item in &transparent_phase.items {
//             let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
//             draw_function.draw(world, &mut tracked_pass, view_entity, item);
//         }
//         Ok(())
//     }
// }
