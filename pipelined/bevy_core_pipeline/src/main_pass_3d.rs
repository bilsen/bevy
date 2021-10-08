use crate::{ClearColor, Transparent3d};
use bevy_ecs::prelude::*;
use bevy_render2::{
    render_graph::{
        GraphContext, NodeRunError, RecordingNodeInput, RecordingNodeOutput, SlotInfo, SlotType,
    },
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{
        LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
        RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::ExtractedView,
};

pub fn main_pass_3d_node(
    In((mut command_encoder, graph)): In<RecordingNodeInput>,
    clear_color: Res<ClearColor>,
    world: &World,
    transparent: Query<&RenderPhase<Transparent3d>>,
) -> RecordingNodeOutput {
    let color_attachment_texture = graph.get_input_texture("color_attachment");
    let depth_texture = graph.get_input_texture("depth");
    let pass_descriptor = RenderPassDescriptor {
        label: Some("main_pass_3d"),
        color_attachments: &[RenderPassColorAttachment {
            view: color_attachment_texture,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(clear_color.0.into()),
                store: true,
            },
        }],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: depth_texture,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.0),
                store: true,
            }),
            stencil_ops: None,
        }),
    };

    let view_entity = *graph.get_input_entity("view");
    let draw_functions = world
        .get_resource::<DrawFunctions<Transparent3d>>()
        .unwrap();

    let transparent_phase = transparent
        .get(view_entity)
        .expect("view entity should exist");

    let render_pass = command_encoder.begin_render_pass(&pass_descriptor);
    let mut draw_functions = draw_functions.write();
    {
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        for item in transparent_phase.items.iter() {
            let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
            draw_function.draw(world, &mut tracked_pass, view_entity, item);
        }
    }

    Ok(command_encoder)
}
