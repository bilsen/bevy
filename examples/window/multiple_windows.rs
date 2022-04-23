use bevy::{
    core_pipeline::{self, AlphaMask3d, Opaque3d, Transparent3d},
    prelude::*,
    render::{
        camera::{ActiveCamera, CameraTypePlugin, RenderTarget},
        render_graph::{
            self, MainRenderGraph, NodeRunError, QueueContext, QueueNode, RenderGraphs, SlotValues,
        },
        render_phase::RenderPhase,
        RenderApp, RenderStage,
    },
    window::{CreateWindow, PresentMode, WindowId},
};

/// This example creates a second window and draws a mesh from two different cameras, one in each window
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(SecondWindowCameraPlugin)
        .add_startup_system(setup)
        .add_startup_system(create_new_window)
        .run();
}

struct SecondWindowCameraPlugin;
impl Plugin for SecondWindowCameraPlugin {
    fn build(&self, app: &mut App) {
        // adds the `ActiveCamera<SecondWindowCamera3d>` resource and extracts the camera into the render world
        app.add_plugin(CameraTypePlugin::<SecondWindowCamera3d>::default());

        let render_app = app.sub_app_mut(RenderApp);

        // add `RenderPhase<Opaque3d>`, `RenderPhase<AlphaMask3d>` and `RenderPhase<Transparent3d>` camera phases
        render_app.add_system_to_stage(RenderStage::Extract, extract_second_camera_phases);

        // add a render graph node that executes the 3d subgraph
        let mut graphs = render_app.world.resource_mut::<RenderGraphs>();
        let main_graph = graphs.get_mut(&MainRenderGraph).unwrap();

        main_graph
            .add_queueing_node("second_window_cam", SecondWindowDriverNode)
            .unwrap();
        main_graph
            .add_edge(
                core_pipeline::node::MAIN_PASS_DEPENDENCIES,
                "second_window_cam",
            )
            .unwrap();
        main_graph
            .add_edge(core_pipeline::node::CLEAR_PASS_DRIVER, "second_window_cam")
            .unwrap();
    }
}

struct SecondWindowDriverNode;
impl render_graph::Node for SecondWindowDriverNode {}

impl QueueNode for SecondWindowDriverNode {
    fn queue(
        &self,
        _slot_values: &render_graph::SlotValues,
        queue_context: &mut QueueContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if let Some(camera) = world.resource::<ActiveCamera<SecondWindowCamera3d>>().get() {
            queue_context.queue(
                core_pipeline::draw_3d_graph::NAME,
                SlotValues::default()
                    .with(core_pipeline::draw_3d_graph::input::VIEW_ENTITY, camera),
            )?;
        }

        Ok(())
    }
}

fn extract_second_camera_phases(
    mut commands: Commands,
    active: Res<ActiveCamera<SecondWindowCamera3d>>,
) {
    if let Some(entity) = active.get() {
        commands.get_or_spawn(entity).insert_bundle((
            RenderPhase::<Opaque3d>::default(),
            RenderPhase::<AlphaMask3d>::default(),
            RenderPhase::<Transparent3d>::default(),
        ));
    }
}

#[derive(Component, Default)]
struct SecondWindowCamera3d;

fn create_new_window(mut create_window_events: EventWriter<CreateWindow>, mut commands: Commands) {
    let window_id = WindowId::new();

    // sends out a "CreateWindow" event, which will be received by the windowing backend
    create_window_events.send(CreateWindow {
        id: window_id,
        descriptor: WindowDescriptor {
            width: 800.,
            height: 600.,
            present_mode: PresentMode::Immediate,
            title: "Second window".to_string(),
            ..default()
        },
    });

    // second window camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        camera: Camera {
            target: RenderTarget::Window(window_id),
            ..default()
        },
        transform: Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        marker: SecondWindowCamera3d,
        ..PerspectiveCameraBundle::new()
    });
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // add entities to the world
    commands.spawn_scene(asset_server.load("models/monkey/Monkey.gltf#Scene0"));
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..default()
    });
    // main camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
