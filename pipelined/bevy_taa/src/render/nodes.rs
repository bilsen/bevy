use std::sync::Mutex;

use bevy_core_pipeline::ViewDepthTexture;
use bevy_ecs::{
    prelude::{QueryState, World},
    system::lifetimeless::Read,
};
use bevy_render2::{
    camera::{CameraPlugin, ExtractedCamera, ExtractedCameraNames},
    color::Color,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{
        Extent3d, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
        RenderPassDescriptor, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage,
        TextureView,
    },
    renderer::{RenderContext, RenderDevice},
    view::ExtractedWindows,
};
use bevy_utils::HashMap;
use bevy_window::WindowId;

use super::Velocity;

struct VelocityTexture {
    width: u32,
    height: u32,
    view: TextureView,
}

impl VelocityTexture {
    pub fn new(ctx: &mut RenderContext, width: u32, height: u32) -> Self {
        let texture = ctx.render_device.create_texture(&TextureDescriptor {
            label: Some("velocity_target"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            format: TextureFormat::Rg16Float,
            dimension: TextureDimension::D2,
            sample_count: 1,
            mip_level_count: 1,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
        });

        VelocityTexture {
            width,
            height,
            view: texture.create_view(&Default::default()),
        }
    }
}

/// Outputs an texture with size equal to the window
/// currently targeted by the [`CameraPlugin::CAMERA_3D`].
#[derive(Default)]
pub struct VelocityTextureNode {
    // NOTE: it might not be worth it cache the textures
    textures: Mutex<HashMap<WindowId, VelocityTexture>>,
    empty_texture: Option<TextureView>,
}

impl VelocityTextureNode {
    pub const VELOCITY_TARGET: &'static str = "VELOCITY_TARGET";
    pub fn new(world: &World) -> Self {
        Self {
            textures: Mutex::new(HashMap::default()),
            empty_texture: None,
        }
    }
}

impl Node for VelocityTextureNode {
    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::VELOCITY_TARGET, SlotType::TextureView)]
    }

    fn update(&mut self, world: &mut World) {
        if self.empty_texture.is_none() {
            let render_device = world.get_resource::<RenderDevice>().unwrap();

            let texture = render_device.create_texture(&TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                format: TextureFormat::Rg16Float,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
            });

            self.empty_texture = Some(texture.create_view(&Default::default()));
        }
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        let extracted_windows = world.get_resource::<ExtractedWindows>().unwrap();

        if let Some(camera_3d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_3D) {
            let extracted_camera = world.entity(*camera_3d).get::<ExtractedCamera>().unwrap();
            let extracted_window = extracted_windows.get(&extracted_camera.window_id).unwrap();

            let mut textures = self.textures.lock().unwrap();

            let velocity_texture = textures.entry(extracted_window.id).or_insert_with(|| {
                VelocityTexture::new(
                    render_context,
                    extracted_window.physical_width,
                    extracted_window.physical_height,
                )
            });

            if velocity_texture.width != extracted_window.physical_width
                || velocity_texture.height != extracted_window.physical_height
            {
                *velocity_texture = VelocityTexture::new(
                    render_context,
                    extracted_window.physical_width,
                    extracted_window.physical_height,
                );
            }

            graph.set_output(Self::VELOCITY_TARGET, velocity_texture.view.clone())?;
        } else {
            graph.set_output(Self::VELOCITY_TARGET, self.empty_texture.clone().unwrap())?;
        }

        Ok(())
    }
}

pub struct TaaVelocityNode {
    query: QueryState<Read<RenderPhase<Velocity>>>,
}

impl TaaVelocityNode {
    pub const VIEW_ENTITY: &'static str = "view_entity";
    pub const TARGET: &'static str = "view_texture";
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for TaaVelocityNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(Self::TARGET, SlotType::TextureView),
            SlotInfo::new(Self::VIEW_ENTITY, SlotType::Entity),
        ]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let velocity_texture = graph.get_input_texture(Self::TARGET).unwrap();
        let view_entity = graph.get_input_entity(Self::VIEW_ENTITY).unwrap();
        let depth_texture = world
            .entity(view_entity)
            .get::<ViewDepthTexture>()
            .expect("View entity should have depth texture");

        let pass_descriptor = RenderPassDescriptor {
            label: Some("taa_velocity"),
            color_attachments: &[RenderPassColorAttachment {
                view: velocity_texture,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK.into()),
                    store: true,
                },
            }],
            // depth_stencil_attachment: None,
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth_texture.view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: false,
                }),
                stencil_ops: None,
            }),
        };

        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        let velocity_phase = self
            .query
            .get_manual(world, view_entity)
            .expect("View entity should exist");

        let draw_functions = world.get_resource::<DrawFunctions<Velocity>>().unwrap();

        let mut draw_functions = draw_functions.write();
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        for item in velocity_phase.items.iter() {
            let draw_function = draw_functions.get_mut(item.draw_velocity).unwrap();
            draw_function.draw(world, &mut tracked_pass, view_entity, item);
        }

        Ok(())
    }
}
