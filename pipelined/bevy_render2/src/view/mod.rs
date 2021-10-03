pub mod window;

pub use window::*;

use crate::{RenderApp, RenderStage, render_component::UniformComponentPlugin, render_resource::DynamicUniformVec, renderer::{RenderDevice, RenderQueue}};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, Vec3};
use bevy_transform::components::GlobalTransform;
use crevice::std140::AsStd140;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .add_system_to_stage(RenderStage::PrePrepare, add_view_uniforms);
        app.add_plugin(UniformComponentPlugin::<ViewUniform>::default());

    }
}

pub struct ExtractedView {
    pub projection: Mat4,
    pub transform: GlobalTransform,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, AsStd140)]
pub struct ViewUniform {
    view_proj: Mat4,
    projection: Mat4,
    world_position: Vec3,
}

fn add_view_uniforms(
    mut commands: Commands,
    mut extracted_views: Query<(Entity, &ExtractedView)>,
) {

    commands.insert_or_spawn_batch(extracted_views.iter().map(|(entity, extracted_view)| {
        let projection = extracted_view.projection;
        let view_uniform = ViewUniform {
            view_proj: projection * extracted_view.transform.compute_matrix().inverse(),
            projection,
            world_position: extracted_view.transform.translation,
        };
        
        (entity, (view_uniform,))
    }).collect::<Vec<_>>());
}
