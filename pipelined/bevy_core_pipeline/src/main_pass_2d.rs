use crate::{ClearColor, Transparent2d};
use bevy_ecs::prelude::*;
use bevy_render2::{
    render_graph::{RecordingNodeInput, RecordingNodeOutput},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor},
};

pub fn main_pass_2d_node(
    In((mut command_encoder, graph)): In<RecordingNodeInput>,
    clear_color: Res<ClearColor>,
    world: &World,
    transparent: Query<&RenderPhase<Transparent2d>>,
) -> RecordingNodeOutput {
    let color_attachment_texture = graph.get_input_texture("color_attachment");
    let pass_descriptor = RenderPassDescriptor {
        label: Some("main_pass_2d"),
        color_attachments: &[RenderPassColorAttachment {
            view: color_attachment_texture,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(clear_color.0.into()),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    };

    let view_entity = *graph.get_input_entity("view");
    let draw_functions = world
        .get_resource::<DrawFunctions<Transparent2d>>()
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
