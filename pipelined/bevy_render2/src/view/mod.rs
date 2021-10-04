pub mod window;

pub use window::*;

use crate::render_component::UniformComponentPlugin;
use bevy_app::{App, Plugin};
use bevy_math::{Mat4, Vec3};
use bevy_transform::components::GlobalTransform;
use crevice::std140::AsStd140;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(UniformComponentPlugin::<ExtractedView, ViewUniform>::new(
            convert_extracted_to_uniform,
        ));
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

fn convert_extracted_to_uniform(view: &ExtractedView) -> ViewUniform {
    let projection = view.projection;
    ViewUniform {
        view_proj: projection * view.transform.compute_matrix().inverse(),
        projection,
        world_position: view.transform.translation,
    }
}
