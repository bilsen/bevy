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
        app.add_plugin(UniformComponentPlugin::<ExtractedView, ViewUniform>::new(extracted_view_to_uniform));
    }
}

#[derive(Clone)]
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

impl ViewUniform {
    pub fn new(view_proj: Mat4, projection: Mat4, world_position: Vec3) -> Self {
        Self {
            view_proj,
            projection,
            world_position,
        }
    }
}

pub fn extracted_view_to_uniform(extracted: &ExtractedView) -> ViewUniform {
    let projection = extracted.projection;
    ViewUniform {
            view_proj: projection * extracted.transform.compute_matrix().inverse(),
            projection,
            world_position: extracted.transform.translation,
    }
}
