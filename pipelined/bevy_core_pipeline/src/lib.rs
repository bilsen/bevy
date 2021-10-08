mod main_pass_2d;
mod main_pass_3d;
mod main_pass_driver;

pub use main_pass_2d::*;
pub use main_pass_3d::*;
pub use main_pass_driver::*;

use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_core::FloatOrd;
use bevy_ecs::prelude::*;
use bevy_render2::{RenderApp, RenderStage, RenderWorld, camera::{ActiveCameras, CameraPlugin}, color::Color, render_graph::{MainRenderGraphId, RenderGraph, RenderGraphs, RenderNodeBuilder, SlotInfo, SlotType, empty_node_system}, render_phase::{sort_phase_system, DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase}, render_resource::*, renderer::RenderDevice, texture::{Image, TextureCache}, view::ExtractedView};

/// Resource that configures the clear color
#[derive(Clone, Debug)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::rgb(0.4, 0.4, 0.4))
    }
}

// Plugins that contribute to the RenderGraph should use the following label conventions:
// 1. Graph modules should have a NAME, input module, and node module (where relevant)
// 2. The "top level" graph is the plugin module root. Just add things like `pub mod node` directly under the plugin module
// 3. "sub graph" modules should be nested beneath their parent graph module

pub mod node {
    pub const MAIN_PASS_DEPENDENCIES: &str = "main_pass_dependencies";
    pub const MAIN_PASS_DRIVER: &str = "main_pass_driver";
    pub const VIEW: &str = "view";
}

pub mod draw_2d_graph {
    pub const NAME: &str = "draw_2d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
        pub const RENDER_TARGET: &str = "render_target";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "main_pass";
    }
}

pub mod draw_3d_graph {
    pub const NAME: &str = "draw_3d";
    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
        pub const RENDER_TARGET: &str = "render_target";
        pub const DEPTH: &str = "depth";
    }
    pub mod node {
        pub const MAIN_PASS: &str = "main_pass";
    }
}

#[derive(Default)]
pub struct CorePipelinePlugin;

impl Plugin for CorePipelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClearColor>();

        let render_app = app.sub_app(RenderApp);
        render_app
            .init_resource::<DrawFunctions<Transparent2d>>()
            .init_resource::<DrawFunctions<Transparent3d>>()
            .add_system_to_stage(RenderStage::Extract, extract_clear_color)
            .add_system_to_stage(RenderStage::Extract, extract_core_pipeline_camera_phases)
            .add_system_to_stage(RenderStage::Prepare, prepare_core_views_system)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent2d>)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Transparent3d>);

        let MainRenderGraphId(main_graph_id_ref) = render_app.world.get_resource().unwrap();
        let main_graph_id = main_graph_id_ref.clone();

        let mut graphs = render_app.world.get_resource_mut::<RenderGraphs>().unwrap();

        let mut draw_2d_graph = RenderGraph::new("draw_2d_graph");
        draw_2d_graph.add_node(RenderNodeBuilder::new()
            .with_name(draw_2d_graph::node::MAIN_PASS)
            .with_system(main_pass_2d_node)
            .build()
        );

        let draw_2d_graph_id = *draw_2d_graph.id();
        graphs.insert(draw_2d_graph::NAME, draw_2d_graph);

        let mut draw_3d_graph = RenderGraph::new("draw_3d_graph");
        
        draw_3d_graph.add_node(RenderNodeBuilder::new()
            .with_name(draw_3d_graph::node::MAIN_PASS)
            .with_system(main_pass_3d_node)
            .build());
        let draw_3d_graph_id = *draw_3d_graph.id();
        graphs.insert(draw_3d_graph::NAME, draw_3d_graph);

        let main_graph = graphs.get_mut(&main_graph_id).unwrap();

        main_graph.add_node(RenderNodeBuilder::new()
        .with_name(node::MAIN_PASS_DRIVER)
        .with_system(main_pass_driver_node_system)
        .build());
        
        main_graph.add_node(RenderNodeBuilder::new()
            .with_name(node::MAIN_PASS_DEPENDENCIES)
            .with_system(empty_node_system)
            .build()
        );
        
        main_graph
            .add_edge(node::MAIN_PASS_DEPENDENCIES, node::MAIN_PASS_DRIVER)
            .unwrap();

        render_app
            .world
            .insert_resource(Draw2dGraphId(draw_2d_graph_id));
        render_app
            .world
            .insert_resource(Draw3dGraphId(draw_3d_graph_id));
    }
}

pub struct Transparent2d {
    pub sort_key: Handle<Image>,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Transparent2d {
    type SortKey = Handle<Image>;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.sort_key.clone_weak()
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

pub struct Transparent3d {
    pub distance: f32,
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Transparent3d {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

pub struct ViewDepthTexture {
    pub texture: Texture,
    pub view: TextureView,
}

pub fn extract_clear_color(clear_color: Res<ClearColor>, mut render_world: ResMut<RenderWorld>) {
    // If the clear color has changed
    if clear_color.is_changed() {
        // Update the clear color resource in the render world
        render_world.insert_resource(clear_color.clone())
    }
}

pub fn extract_core_pipeline_camera_phases(
    mut commands: Commands,
    active_cameras: Res<ActiveCameras>,
) {
    if let Some(camera_2d) = active_cameras.get(CameraPlugin::CAMERA_2D) {
        if let Some(entity) = camera_2d.entity {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Transparent2d>::default());
        }
    }
    if let Some(camera_3d) = active_cameras.get(CameraPlugin::CAMERA_3D) {
        if let Some(entity) = camera_3d.entity {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Transparent3d>::default());
        }
    }
}

pub fn prepare_core_views_system(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedView), With<RenderPhase<Transparent3d>>>,
) {
    for (entity, view) in views.iter() {
        let cached_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("view_depth_texture"),
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: view.width as u32,
                    height: view.height as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Depth32Float, /* PERF: vulkan docs recommend using 24
                                                      * bit depth for better performance */
                usage: TextureUsage::RENDER_ATTACHMENT,
            },
        );
        commands.entity(entity).insert(ViewDepthTexture {
            texture: cached_texture.texture,
            view: cached_texture.default_view,
        });
    }
}
