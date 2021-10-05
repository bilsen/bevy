pub mod nodes;

use bevy_asset::Handle;
use bevy_core::FloatOrd;
use bevy_ecs::{
    entity::Entity,
    prelude::{FromWorld, With, World},
    system::{
        lifetimeless::{Read, SQuery, SRes},
        Commands, Query, Res, SystemParamItem,
    },
};
use bevy_pbr2::{DrawMesh, MeshUniform};
use bevy_render2::{
    camera::{ActiveCameras, CameraPlugin},
    mesh::Mesh,
    render_asset::RenderAssets,
    render_component::{ComponentUniforms, DynamicUniformIndex},
    render_phase::{
        DrawFunctionId, DrawFunctions, PhaseItem, RenderCommand, RenderPhase, TrackedRenderPass,
    },
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState,
        BufferBindingType, BufferSize, ColorTargetState, ColorWrite, CompareFunction,
        DepthBiasState, DepthStencilState, Face, FragmentState, FrontFace, IndexFormat,
        InputStepMode, MultisampleState, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
        PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderStage,
        StencilFaceState, StencilState, TextureFormat, VertexAttribute, VertexBufferLayout,
        VertexFormat, VertexState,
    },
    renderer::RenderDevice,
    shader::Shader,
    view::{ExtractedView, ViewUniform},
};

use crate::save::Previous;

pub struct TaaVelocityShaders {
    pub pipeline: RenderPipeline,
    pub shader_module: ShaderModule,

    // transform, previous transform
    pub view_layout: BindGroupLayout,
    // transform, previous transform
    pub mesh_layout: BindGroupLayout,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for TaaVelocityShaders {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let shader = Shader::from_wgsl(include_str!("taa.wgsl"));
        let shader_module = render_device.create_shader_module(&shader);

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View transform
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
                // previous
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to PreviousViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let mesh_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to MeshUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: change this to MeshUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(144),
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[&view_layout, &mesh_layout],
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

        TaaVelocityShaders {
            pipeline,
            shader_module,
            view_layout,
            mesh_layout,
        }
    }
}

pub struct TaaMeshBindGroup {
    pub value: BindGroup,
}

pub fn queue_taa_mesh_bind_group(
    mut commands: Commands,
    taa_shader: Res<TaaVelocityShaders>,
    render_device: Res<RenderDevice>,
    mesh_uniforms: Res<ComponentUniforms<MeshUniform>>,
    previous_mesh_uniforms_option: Option<Res<ComponentUniforms<Previous<MeshUniform>>>>,
) {
    if previous_mesh_uniforms_option.is_none() {
        return;
    }
    let previous_mesh_uniforms = previous_mesh_uniforms_option.unwrap();

    if let (Some(binding), Some(binding2)) = (
        mesh_uniforms.uniforms().binding(),
        previous_mesh_uniforms.uniforms().binding(),
    ) {
        commands.insert_resource(TaaMeshBindGroup {
            value: render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: binding,
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: binding2,
                    },
                ],
                label: None,
                layout: &taa_shader.mesh_layout,
            }),
        });
    }
}

pub struct TaaViewBindGroup {
    pub value: BindGroup,
}

pub fn queue_taa_view_bind_group(
    mut commands: Commands,
    taa_shader: Res<TaaVelocityShaders>,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ComponentUniforms<ViewUniform>>,
    previous_view_uniforms_option: Option<Res<ComponentUniforms<Previous<ViewUniform>>>>,
) {

    if previous_view_uniforms_option.is_none() {
        return;
    }
    let previous_view_uniforms = previous_view_uniforms_option.unwrap();

    if let (Some(binding), Some(binding2)) =
        (view_uniforms.binding(), previous_view_uniforms.binding())
    {
        commands.insert_resource(TaaViewBindGroup {
            value: render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: binding,
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: binding2,
                    },
                ],
                label: None,
                layout: &taa_shader.view_layout,
            }),
        });
    }
}

pub fn queue_velocity(
    velocity_draw_functions: Res<DrawFunctions<Velocity>>,
    meshes: Query<(Entity, &MeshUniform), (With<Handle<Mesh>>, With<Previous<MeshUniform>>)>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Velocity>)>,
) {
    let draw_velocity = velocity_draw_functions
        .read()
        .get_id::<DrawTaaVelocity>()
        .unwrap();

    let mut num_queued = 0;
    for (view, mut velocity_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_uniform) in meshes.iter() {
            velocity_phase.add(Velocity {
                entity,
                distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                draw_velocity,
            });
            num_queued += 1;
        }
    }
    println!("Num queued {}", num_queued);
}

pub fn extract_velocity_camera_phases(mut commands: Commands, active_cameras: Res<ActiveCameras>) {
    if let Some(camera_3d) = active_cameras.get(CameraPlugin::CAMERA_3D) {
        if let Some(entity) = camera_3d.entity {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Velocity>::default());
        }
    }
}

pub struct Velocity {
    pub distance: f32,
    entity: Entity,
    draw_velocity: DrawFunctionId,
}
impl PhaseItem for Velocity {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_velocity
    }
}

pub type DrawTaaVelocity = (
    SetVelocityPipeline,
    SetTaaMeshBindGroup<0>,
    SetTaaViewBindGroup<1>,
    DrawMesh,
);

pub struct SetVelocityPipeline;

impl RenderCommand<Velocity> for SetVelocityPipeline {
    type Param = (SRes<TaaVelocityShaders>,);

    fn render<'w>(
        view: Entity,
        item: &Velocity,
        (shaders,): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        pass.set_render_pipeline(&shaders.into_inner().pipeline);
    }
}

pub struct SetTaaMeshBindGroup<const I: usize>;

impl<const I: usize> RenderCommand<Velocity> for SetTaaMeshBindGroup<I> {
    type Param = (
        SQuery<(
            Read<DynamicUniformIndex<MeshUniform>>,
            Read<DynamicUniformIndex<Previous<MeshUniform>>>,
        )>,
        SRes<TaaMeshBindGroup>,
    );

    fn render<'w>(
        view: Entity,
        item: &Velocity,
        (query, bind_group): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let (mesh_index, previous_mesh_index) = query.get(item.entity).unwrap();
        pass.set_bind_group(
            I,
            &bind_group.into_inner().value,
            &[mesh_index.index(), previous_mesh_index.index()],
        );
    }
}

pub struct SetTaaViewBindGroup<const I: usize>;

impl<const I: usize> RenderCommand<Velocity> for SetTaaViewBindGroup<I> {
    type Param = (
        SQuery<(
            Read<DynamicUniformIndex<ViewUniform>>,
            Read<DynamicUniformIndex<Previous<ViewUniform>>>,
        )>,
        SRes<TaaViewBindGroup>,
    );

    fn render<'w>(
        view: Entity,
        item: &Velocity,
        (query, bind_group): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        
        let (view_index, previous_view_index) = query.get(view).unwrap();
        pass.set_bind_group(
            I,
            &bind_group.into_inner().value,
            &[view_index.index(), previous_view_index.index()],
        );
    }
}

impl RenderCommand<Velocity> for DrawMesh {
    type Param = (SRes<RenderAssets<Mesh>>, SQuery<Read<Handle<Mesh>>>);
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: &Velocity,
        (meshes, mesh_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) {
        let mesh_handle = mesh_query.get(item.entity).unwrap();
        let gpu_mesh = meshes.into_inner().get(mesh_handle).unwrap();
        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        if let Some(index_info) = &gpu_mesh.index_info {
            pass.set_index_buffer(index_info.buffer.slice(..), 0, IndexFormat::Uint32);
            pass.draw_indexed(0..index_info.count, 0, 0..1);
        } else {
            panic!("non-indexed drawing not supported yet")
        }
    }
}
