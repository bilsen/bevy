
use std::sync::Mutex;

use bevy_asset::Handle;
use bevy_core::FloatOrd;
use bevy_core_pipeline::ViewDepthTexture;
use bevy_ecs::{entity::Entity, prelude::{FromWorld, With, World}, system::{Commands, Query, Res, ResState, SystemParamItem, lifetimeless::{Read, SQuery, SRes}}};
use bevy_pbr2::{DrawMesh, MeshUniform};
use bevy_render2::{camera::{ActiveCameras, CameraPlugin}, color::Color, mesh::Mesh, render_asset::RenderAssets, render_component::{ComponentUniforms, DynamicUniformIndex}, render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType}, render_phase::{
        DrawFunctionId, DrawFunctions, PhaseItem, RenderCommand, RenderPhase, TrackedRenderPass,
    }, render_resource::{AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferSize, ColorTargetState, ColorWrite, CompareFunction, DepthBiasState, DepthStencilState, Extent3d, Face, FilterMode, FragmentState, FrontFace, IndexFormat, InputStepMode, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderModule, ShaderStage, StencilFaceState, StencilState, Texture, TextureFormat, TextureSampleType, TextureView, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState}, renderer::{RenderContext, RenderDevice}, shader::Shader, texture::TextureCache, view::{ExtractedView, ViewUniform}};
use bevy_utils::HashMap;
use bevy_window::WindowId;
use wgpu::{ImageCopyTexture, Origin3d, TextureDescriptor, TextureDimension, TextureUsage};
use bevy_ecs::prelude::ResMut;
use bevy_render2::camera::ExtractedCameraNames;
use bevy_render2::render_resource::{BindingResource, RenderPassDepthStencilAttachment};
use bevy_render2::view::ExtractedWindows;

use crate::save::Previous;

use super::Velocity;

pub struct TaaResolveShaders {
    pub pipeline: RenderPipeline,
    pub shader_module: ShaderModule,
    pub layout: BindGroupLayout,
    // pub sampler: Sampler
}
/// aSD
// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for TaaResolveShaders {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let shader = Shader::from_wgsl(include_str!("resolve.wgsl"));
        let shader_module = render_device.create_shader_module(&shader);
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("taa_resolve_layout"),
            entries: &[
                // color attachment
                BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStage::VERTEX_FRAGMENT,
                    count: None,
                },
                // depth attachment
                BindGroupLayoutEntry {
                    binding: 2,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStage::VERTEX_FRAGMENT,
                    count: None,
                },
                // velocity attachment
                BindGroupLayoutEntry {
                    binding: 4,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStage::VERTEX_FRAGMENT,
                    count: None,
                },
                // previous color attachment
                BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStage::VERTEX_FRAGMENT,
                    count: None,
                },
                // previous depth
                BindGroupLayoutEntry {
                    binding: 2,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStage::VERTEX_FRAGMENT,
                    count: None,
                },
                // sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    visibility: ShaderStage::VERTEX_FRAGMENT,
                    count: None,
                },
            ],
        });



        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[&layout],
        });

        let pipeline = render_device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            vertex: VertexState {
                buffers: &[VertexBufferLayout {
                    array_stride: 0,
                    step_mode: InputStepMode::Vertex,
                    attributes: &[],
                }],
                module: &shader_module,
                entry_point: "vertex",
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment",
                targets: &[ColorTargetState {
                    format: TextureFormat::Rg16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::Zero,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrite::ALL,
                }],
            }),
            // depth_stencil: None,
            depth_stencil: None,
            layout: Some(&pipeline_layout),
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
        });


        
        TaaResolveShaders {
            pipeline,
            shader_module,
            layout
        }
    }
}



/*
Shader should take inputs
TaaHistoryDepth
TaaHistoryColor
TargetTexture
TargetDepth
Velocity

and output to

TaaNewHistoryColor
TaaNewHistoryDepth
*/





#[derive(Default)]
pub struct TaaResolveNode;

impl TaaResolveNode {
    pub const TARGET: &'static str = "TARGET";
    pub const INPUT_VIEW: &'static str = "INPUT_VIEW";

}

///
/// Resources:
/// * Res<TaaHistoryColor>
/// * Res<TaaHistoryDepth>
/// Scratch:
/// One texture to copy the input color to, so that it may be rendered to

impl Node for TaaResolveNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(Self::TARGET, SlotType::TextureView),
            SlotInfo::new(Self::INPUT_VIEW, SlotType::Entity)
        ]
    }

    // fn update(&mut self, world: &mut World) {
    //     self.query.update_archetypes(world);
    // }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let color_texture = graph.get_input_texture(Self::TARGET).unwrap();
        let view_entity = graph.get_input_entity(Self::INPUT_VIEW).unwrap();
        let depth_texture = world
            .entity(view_entity)
            .get::<ViewDepthTexture>()
            .expect("View entity should have depth texture");

        let history_depth = world.get_resource::<TaaHistoryDepth>().unwrap();
        let history_color = world.get_resource::<TaaHistoryColor>().unwrap();

        let new_history_depth = world.get_resource::<TaaNewHistoryDepth>().unwrap();
        let new_history_color = world.get_resource::<TaaNewHistoryColor>().unwrap();

        let shaders = world.get_resource::<TaaResolveShaders>().unwrap();


        let depth_sampler = render_context.render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareFunction::GreaterEqual),
            ..Default::default()
        });

        let color_sampler = render_context.render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareFunction::GreaterEqual),
            ..Default::default()
        });

        let bind_group = render_context.render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &color_texture,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &history_color.view,
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &history_depth.view,
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(
                        &depth_texture.view,
                    ),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(
                        &depth_sampler,
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(
                        &color_sampler,
                    ),
                },
            ],
            label: Some("taa_resolve_bind_group"),
            layout: &shaders.layout,
        });


        // Render to TaaNewHistoryColor
        let pass_descriptor = RenderPassDescriptor {
            label: Some("taa_resolve"),
            color_attachments: &[RenderPassColorAttachment {
                view: &new_history_color.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK.into()),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(
                RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: false,
                    }),
                    stencil_ops: None,
                }
            ),
        };

        let extracted_view = world.entity(view_entity).get::<ExtractedView>().unwrap();
        let width = extracted_view.width;
        let height = extracted_view.height;
        // render_context.command_encoder.copy_texture_to_texture(
        //     ImageCopyTexture {
        //         T
        //     }, ImageCopyTexture {
        //
        //     }, Extent3d {
        //         width,
        //         height,
        //         depth_or_array_layers: 1
        //     }
        // );
        //

        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);

        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        tracked_pass.set_render_pipeline(&shaders.pipeline);
        tracked_pass.set_bind_group(0, &bind_group, &[]);
        tracked_pass.draw(0..3, 0..1);



        Ok(())
    }
}


struct TaaHistoryDepth {
    texture: Texture,
    view: TextureView,
}

struct TaaHistoryColor {
    texture: Texture,
    view: TextureView,
}

struct TaaNewHistoryDepth {
    texture: Texture,
    view: TextureView,
}

struct TaaNewHistoryColor {
    texture: Texture,
    view: TextureView,
}


pub fn prepare_taa(
    mut commands: Commands,
    camera_names: Res<ExtractedCameraNames>,
    windows: Res<ExtractedWindows>,
    cache: ResMut<TextureCache>,
    device: Res<RenderDevice>,
) {
    for (name, entity) in camera_names.entities {
        windows.get(entity);

        let depth = cache.get(device, TextureDescriptor {
            label: "history_depth".into(),
            size: Extent3d {
                width: 0,
                height: 0,
                depth_or_array_layers: 0
            },
            mip_level_count: 0,
            sample_count: 0,
            dimension: TextureDimension::D1,
            format: TextureFormat::R8Unorm,
            usage: TextureUsage::SAMPLED | TextureUsage::RENDER_ATTACHMENT
        });


        commands.entity(entity).insert_bundle(
            (
                    TaaHistoryDepth {
                        texture:
                    }
                )
        )
    }
}

pub fn taa_swap_system(
    new_color: ResMut<TaaNewHistoryColor>,
    new_depth: ResMut<TaaNewHistoryDepth>,
    mut color: ResMut<TaaHistoryColor>,
    mut depth: ResMut<TaaHistoryDepth>,
) {

    color.texture = new_color.texture.clone();
    color.view = new_color.view.clone();

    depth.texture = new_depth.texture.clone();
    depth.view = new_depth.view.clone();

}
