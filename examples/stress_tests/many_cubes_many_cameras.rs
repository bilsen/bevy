use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::{DVec2, DVec3},
    prelude::*, render::{camera::{CameraTypePlugin, ActiveCamera, RenderTarget}, RenderApp, RenderStage, render_graph::{RenderGraphs, MainRenderGraph, QueueNode, self, QueueContext, NodeRunError, SlotValues}, render_phase::RenderPhase}, core_pipeline::{self, Opaque3d, AlphaMask3d, Transparent3d}, window::{WindowId, CreateWindow, PresentMode},
};
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(SecondWindowCameraPlugin)
        .add_startup_system(setup)
        .add_system(move_camera)
        .add_system(print_mesh_count)
        .run();
}

fn setup(
    mut create_window_events: EventWriter<CreateWindow>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const WIDTH: usize = 50;
    const HEIGHT: usize = 50;
    let mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let material = materials.add(StandardMaterial {
        base_color: Color::PINK,
        ..default()
    });
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


    match std::env::args().nth(1).as_deref() {
        Some("sphere") => {
            // NOTE: This pattern is good for testing performance of culling as it provides roughly
            // the same number of visible meshes regardless of the viewing angle.
            const N_POINTS: usize = WIDTH * HEIGHT * 4;
            // NOTE: f64 is used to avoid precision issues that produce visual artifacts in the distribution
            let radius = WIDTH as f64 * 2.5;
            let golden_ratio = 0.5f64 * (1.0f64 + 5.0f64.sqrt());
            for i in 0..N_POINTS {
                let spherical_polar_theta_phi =
                    fibonacci_spiral_on_sphere(golden_ratio, i, N_POINTS);
                let unit_sphere_p = spherical_polar_to_cartesian(spherical_polar_theta_phi);
                commands.spawn_bundle(PbrBundle {
                    mesh: mesh.clone_weak(),
                    material: material.clone_weak(),
                    transform: Transform::from_translation((radius * unit_sphere_p).as_vec3()),
                    ..default()
                });
            }

            // camera
            commands.spawn_bundle(PerspectiveCameraBundle::default());
            
        }
        _ => {
            // NOTE: This pattern is good for demonstrating that frustum culling is working correctly
            // as the number of visible meshes rises and falls depending on the viewing angle.
            for x in 0..WIDTH {
                for y in 0..HEIGHT {
                    // introduce spaces to break any kind of moiré pattern
                    if x % 10 == 0 || y % 10 == 0 {
                        continue;
                    }
                    // cube
                    commands.spawn_bundle(PbrBundle {
                        mesh: mesh.clone_weak(),
                        material: material.clone_weak(),
                        transform: Transform::from_xyz((x as f32) * 2.5, (y as f32) * 2.5, 0.0),
                        ..default()
                    });
                    commands.spawn_bundle(PbrBundle {
                        mesh: mesh.clone_weak(),
                        material: material.clone_weak(),
                        transform: Transform::from_xyz(
                            (x as f32) * 2.5,
                            HEIGHT as f32 * 2.5,
                            (y as f32) * 2.5,
                        ),
                        ..default()
                    });
                    commands.spawn_bundle(PbrBundle {
                        mesh: mesh.clone_weak(),
                        material: material.clone_weak(),
                        transform: Transform::from_xyz((x as f32) * 2.5, 0.0, (y as f32) * 2.5),
                        ..default()
                    });
                    commands.spawn_bundle(PbrBundle {
                        mesh: mesh.clone_weak(),
                        material: material.clone_weak(),
                        transform: Transform::from_xyz(0.0, (x as f32) * 2.5, (y as f32) * 2.5),
                        ..default()
                    });
                }
            }
            // camera
            commands.spawn_bundle(PerspectiveCameraBundle {
                transform: Transform::from_xyz(WIDTH as f32, HEIGHT as f32, WIDTH as f32),
                ..default()
            });
            commands.spawn_bundle(PerspectiveCameraBundle {
                camera: Camera {
                    target: RenderTarget::Window(window_id),
                    ..default()
                },
                transform: Transform::from_xyz(WIDTH as f32, HEIGHT as f32, WIDTH as f32).with_rotation(Quat::from_rotation_x(50.)),
                marker: SecondWindowCamera3d,
                ..PerspectiveCameraBundle::new()
            });
        }
    }

    // add one cube, the only one with strong handles
    // also serves as a reference point during rotation
    commands.spawn_bundle(PbrBundle {
        mesh,
        material,
        transform: Transform {
            translation: Vec3::new(0.0, HEIGHT as f32 * 2.5, 0.0),
            scale: Vec3::splat(5.0),
            ..default()
        },
        ..default()
    });

    commands.spawn_bundle(DirectionalLightBundle { ..default() });
}

// NOTE: This epsilon value is apparently optimal for optimizing for the average
// nearest-neighbor distance. See:
// http://extremelearning.com.au/how-to-evenly-distribute-points-on-a-sphere-more-effectively-than-the-canonical-fibonacci-lattice/
// for details.
const EPSILON: f64 = 0.36;
fn fibonacci_spiral_on_sphere(golden_ratio: f64, i: usize, n: usize) -> DVec2 {
    DVec2::new(
        2.0 * std::f64::consts::PI * (i as f64 / golden_ratio),
        (1.0 - 2.0 * (i as f64 + EPSILON) / (n as f64 - 1.0 + 2.0 * EPSILON)).acos(),
    )
}

fn spherical_polar_to_cartesian(p: DVec2) -> DVec3 {
    let (sin_theta, cos_theta) = p.x.sin_cos();
    let (sin_phi, cos_phi) = p.y.sin_cos();
    DVec3::new(cos_theta * sin_phi, sin_theta * sin_phi, cos_phi)
}

// System for rotating the camera
fn move_camera(time: Res<Time>, mut camera_query: Query<&mut Transform, With<Camera>>) {
    for mut camera_transform in camera_query.iter_mut() {
        camera_transform.rotate(Quat::from_rotation_z(time.delta_seconds() * 0.15));
        camera_transform.rotate(Quat::from_rotation_x(time.delta_seconds() * 0.15));
    }
}

// System for printing the number of meshes on every tick of the timer
fn print_mesh_count(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    sprites: Query<(&Handle<Mesh>, &ComputedVisibility)>,
) {
    timer.tick(time.delta());

    if timer.just_finished() {
        info!(
            "Meshes: {} - Visible Meshes {}",
            sprites.iter().len(),
            sprites.iter().filter(|(_, cv)| cv.is_visible).count(),
        );
    }
}

#[derive(Deref, DerefMut)]
struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, true))
    }
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


