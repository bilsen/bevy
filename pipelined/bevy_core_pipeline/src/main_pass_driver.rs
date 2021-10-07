use crate::ViewDepthTexture;
use bevy_ecs::{system::Res, world::World};
use bevy_render2::{camera::{CameraPlugin, ExtractedCamera, ExtractedCameraNames}, render_graph::{GraphContext, NodeInput, NodeResult, NodeRunError, RenderGraph, RenderGraphId, RunSubGraphs, SlotValue}, renderer::RenderContext, view::ExtractedWindows};
use bevy_ecs::prelude::*;

use bevy_ecs::system::In;


pub struct Draw2dGraphId(pub RenderGraphId);
pub struct Draw3dGraphId(pub RenderGraphId);


pub fn main_pass_driver_node_system(
    In((mut render_context, graph)): In<NodeInput>,
    extracted_camera_names: Res<ExtractedCameraNames>,
    extracted_windows: Res<ExtractedWindows>,
    extracted_cameras: Query<&ExtractedCamera>,
    depth_textures: Query<&ViewDepthTexture>,
    draw_2d_id: Res<Draw2dGraphId>,
    draw_3d_id: Res<Draw3dGraphId>

) -> NodeResult {

    let mut sub_graph_runs = RunSubGraphs::default();

    if let Some(camera_2d) = extracted_camera_names.entities.get(CameraPlugin::CAMERA_2D) {
        let extracted_camera = extracted_cameras.get(*camera_2d).unwrap();
        let extracted_window = extracted_windows.get(&extracted_camera.window_id).unwrap();
        let swap_chain_texture = extracted_window.swap_chain_frame.as_ref().unwrap().clone();
        sub_graph_runs.run(
            draw_2d_id.0,
            vec![
                ("view", SlotValue::Entity(*camera_2d)),
                ("color_attachment", SlotValue::TextureView(swap_chain_texture))
            ],
        );
    }

    if let Some(camera_3d) = extracted_camera_names.entities.get(CameraPlugin::CAMERA_3D) {
        let extracted_camera = extracted_cameras.get(*camera_3d).unwrap();
        let depth_texture = depth_textures.get(*camera_3d).unwrap();
        let extracted_window = extracted_windows.get(&extracted_camera.window_id).unwrap();
        let swap_chain_texture = extracted_window.swap_chain_frame.as_ref().unwrap().clone();
        sub_graph_runs.run(
            draw_3d_id.0,
            vec![
                ("view", SlotValue::Entity(*camera_3d)),
                ("color_attachment", SlotValue::TextureView(swap_chain_texture)),
                ("depth", SlotValue::TextureView(depth_texture.view.clone())),
            ],
        );
    }

    Ok((render_context, sub_graph_runs))
}

