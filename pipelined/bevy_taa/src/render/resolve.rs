
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

use crate::save::Previous;

use super::Velocity;

pub struct TaaResolveShaders {
    pub pipeline: RenderPipeline,
    pub shader_module: ShaderModule,

    pub current_layout: BindGroupLayout,
    pub history_layout: BindGroupLayout,
    pub sampler: Sampler
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for TaaResolveShaders {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let shader = Shader::from_wgsl(include_str!("resolve.wgsl"));
        let shader_module = render_device.create_shader_module(&shader);
        let current_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("taa_resolve_current_layout"),
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
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Sampler {
                        filtering: true,
                        comparison: false,
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
                BindGroupLayoutEntry {
                    binding: 3,
                    ty: BindingType::Sampler {
                        filtering: true,
                        comparison: false,
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
                BindGroupLayoutEntry {
                    binding: 5,
                    ty: BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    visibility: ShaderStage::VERTEX_FRAGMENT,
                    count: None,
                },
            ],
        });

        let history_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("taa_resolve_current_layout"),
            entries: &[
                // old color attachment
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
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    visibility: ShaderStage::VERTEX_FRAGMENT,
                    count: None,
                },
                // old depth attachment
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
                BindGroupLayoutEntry {
                    binding: 3,
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
            bind_group_layouts: &[&current_layout, &history_layout],
        });

        let pipeline = render_device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            vertex: VertexState {
                buffers: &[VertexBufferLayout {
                    array_stride: 32,
                    step_mode: InputStepMode::Vertex,
                    attributes: &[
                        // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 0,
                        },
                        // Normal
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 1,
                        },
                        // Uv
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 24,
                            shader_location: 2,
                        },
                    ],
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
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
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

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });
        
        TaaResolveShaders {
            pipeline,
            shader_module,
            current_layout,
            history_layout,
            sampler,
        }
    }
}






struct TaaScratchTexture {
    texture: Texture,
    view: TextureView,
}
struct TaaHistoryDepth {
    texture: Texture,
    view: TextureView,
}
struct TaaHistoryColor {
    texture: Texture,
    view: TextureView,
}


#[derive(Default)]
pub struct TaaResolveNode;

impl TaaResolveNode {
    pub const TARGET: &'static str = "TARGET";
    pub const INPUT_VIEW: &'static str = "INPUT_VIEW";
    pub const INPUT_SCRATCH: &'static str = "SCRATCH";
    
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
            SlotInfo::new(Self::INPUT_VIEW, SlotType::Entity),
            SlotInfo::new(Self::INPUT_SCRATCH, SlotType::TextureView)
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
        let scratch = world.get_resource::<TaaScratchTexture>().unwrap();


        let pass_descriptor = RenderPassDescriptor {
            label: Some("taa_resolve"),
            color_attachments: &[RenderPassColorAttachment {
                view: &color_texture,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK.into()),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        };
        
        let extracted_view = world.entity(view_entity).get::<ExtractedView>().unwrap();
        let width = extracted_view.width;
        let height = extracted_view.height;
        render_context.command_encoder.copy_texture_to_texture(
            ImageCopyTexture {
                T
            }, ImageCopyTexture {

            }, Extent3d {
                width,
                height,
                depth_or_array_layers: 1
            }
        )

        render_context.command_encoder.


        let render_pass = render_context
            .command_encoder
            .begin_render_pass(&pass_descriptor);



        Ok(())
    }
}









#[derive(Default)]
pub struct RgbTextureNode;

impl RgbTextureNode {
    pub const RGB_TARGET: &'static str = "rgb_target";
    pub const VIEW_ENTITY: &'static str = "view";
}

impl Node for RgbTextureNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::VIEW_ENTITY, SlotType::Entity)]
    }

    fn output(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::RGB_TARGET, SlotType::TextureView)]
    }

    fn update(&mut self, world: &mut World) {
        
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {

        let entity = graph.get_input_entity(Self::VIEW_ENTITY).unwrap();
        let extracted_view = world.entity(entity).get::<ExtractedView>().unwrap();
        let extent = Extent3d {
            width: extracted_view.width,
            height: extracted_view.height,
            ..Default::default()
        };
        let cache = world.get_resource::<TextureCache>().unwrap();

        let texture = cache.get(&render_context.render_device, TextureDescriptor {
            label: Some("rgb_texture"),
            size: extent,
            format: TextureFormat::Rgba8Unorm,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
        });
        

        graph.set_output(Self::RGB_TARGET, texture.default_view)?;
    

        Ok(())
    }
}


