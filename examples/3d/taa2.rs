use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

use bevy::{PipelinedDefaultPlugins, core::{FloatOrd, Time}, core_pipeline::{CorePipelinePlugin, Transparent3d, ViewDepthTexture}, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, ecs::{
        component::Component,
        prelude::*,
        system::{
            lifetimeless::{Read, SQuery, SRes},
            SystemParamItem,
        },
    }, input::Input, math::{Mat4, Quat, Vec3}, pbr2::{
        AmbientLight, DirectionalLight, DirectionalLightBundle, DrawMesh, MeshUniform, PbrBundle,
        PointLight, PointLightBundle, StandardMaterial,
    }, prelude::{
        App, Assets, BuildChildren, CoreStage, Draw, GlobalTransform, Handle, KeyCode, Plugin,
        Transform,
    }, render::draw, render2::{RenderApp, RenderStage, camera::{ActiveCameras, CameraPlugin, ExtractedCamera, ExtractedCameraNames, OrthographicProjection, PerspectiveCameraBundle}, color::Color, mesh::{shape, Mesh}, render_asset::RenderAssets, render_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin}, render_graph::{
            Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType, SlotValue,
        }, render_phase::{
            AddRenderCommand, DrawFunctionId, DrawFunctions, PhaseItem, RenderCommand, RenderPhase,
            TrackedRenderPass,
        }, render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendComponent,
            BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferSize,
            ColorTargetState, ColorWrite, CompareFunction, DepthBiasState, DepthStencilState,
            DynamicUniformVec, Extent3d, Face, FragmentState, FrontFace, IndexFormat,
            InputStepMode, LoadOp, MultisampleState, Operations, PipelineLayoutDescriptor,
            PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
            RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
            RenderPipelineDescriptor, ShaderModule, ShaderStage, StencilFaceState, StencilState,
            TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsage,
            TextureView, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
            VertexState,
        }, renderer::{RenderContext, RenderDevice, RenderQueue}, shader::Shader, texture::{BevyDefault, Image, TextureCache, TextureFormatPixelInfo}, view::{ExtractedView, ExtractedWindows, ViewUniform, ViewUniformOffset, ViewUniforms}}, utils::HashMap, window::WindowId};

use crevice::std140::{AsStd140, Std140};
use serde::__private::PhantomData;

struct SaveComponentPlugin<C: Component + Clone> {
    _marker: PhantomData<fn() -> C>,
}

impl<C: Component + Clone> Default for SaveComponentPlugin<C> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

#[derive(Clone)]
struct Previous<C: Component>(C);

impl<C: Component + AsStd140> AsStd140 for Previous<C> {
    type Std140Type = C::Std140Type;
    fn as_std140(&self) -> Self::Std140Type {
        self.0.as_std140()
    }
    fn std140_size_static() -> usize {
        C::std140_size_static()
    }

    fn from_std140(val: Self::Std140Type) -> Self {
        Self(C::from_std140(val))
    }
}

impl<C: Component> Deref for Previous<C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<C: Component> DerefMut for Previous<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

struct Saved<C: Component> {
    values: HashMap<Entity, C>,
}

impl<C: Component> Default for Saved<C> {
    fn default() -> Self {
        Self {
            values: HashMap::default(),
        }
    }
}

impl<C: Component + Clone> Plugin for SaveComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        app.sub_app(RenderApp)
            .insert_resource(Saved::<C>::default())
            .add_system_to_stage(RenderStage::Cleanup, save_component_system::<C>)
            .add_system_to_stage(
                RenderStage::Prepare,
                add_saved_components_system::<C>
                    .exclusive_system()
                    .at_start(),
            );
    }
}

fn save_component_system<C: Component + Clone>(
    query: Query<(Entity, &C)>,
    mut saved_resource: ResMut<Saved<C>>,
) {
    for (entity, component) in query.iter() {
        saved_resource.values.insert(entity, component.clone());
    }
}

fn add_saved_components_system<C: Component>(
    mut commands: Commands,
    mut saved_resouce: ResMut<Saved<C>>,
) {
    println!("Adding saved components");
    let bundles_iter: Vec<(Entity, (Previous<C>,))> = saved_resouce
        .into_inner()
        .values
        .drain()
        .map(|(entity, component)| (entity, (Previous(component),)))
        .collect();

    commands.insert_or_spawn_batch(bundles_iter);
}

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

struct TaaPlugin;

impl Plugin for TaaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(SaveComponentPlugin::<MeshUniform>::default())
        .add_plugin(UniformComponentPlugin::<Previous<MeshUniform>>::default());
        
        let render_app = app.sub_app(RenderApp);
        render_app
        .add_system_to_stage(RenderStage::Extract, extract_velocity_camera_phases)
            .add_system_to_stage(RenderStage::Queue, queue_taa_mesh_bind_group)
            .add_system_to_stage(RenderStage::Queue, queue_taa_view_bind_group)
            .add_system_to_stage(RenderStage::Queue, queue_velocity)
            .add_system_to_stage(RenderStage::Cleanup, save_view_uniform)
            .add_system_to_stage(RenderStage::Prepare, add_previous_view_offsets.exclusive_system().at_start())
            .init_resource::<DrawFunctions<Velocity>>()
            .init_resource::<PreviousViewUniformOffsets>()
            .init_resource::<PreviousViewUniforms>()
            .init_resource::<TaaVelocityShaders>();

        render_app.add_render_command::<Velocity, DrawTaaVelocity>();

        let velocity_node = TaaVelocityNode::new(&mut render_app.world);
        let velocity_texture_node = VelocityTextureNode::new(&render_app.world);

        let render_world = render_app.world.cell();

        let mut graph = render_world.get_resource_mut::<RenderGraph>().unwrap();
        let draw_3d_graph = graph.get_sub_graph_mut("draw_3d").unwrap();
        draw_3d_graph.add_node("taa_velocity", velocity_node);
        draw_3d_graph.add_node("taa_velocity_texture", velocity_texture_node);
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                "view_entity",
                "taa_velocity",
                TaaVelocityNode::VIEW_ENTITY,
            )
            .unwrap();

        draw_3d_graph
            .add_slot_edge(
                "taa_velocity_texture",
                VelocityTextureNode::VELOCITY_TARGET,
                "taa_velocity",
                TaaVelocityNode::TARGET,
            )
            .unwrap();
        draw_3d_graph
            .add_node_edge("main_pass", "taa_velocity")
            .unwrap();
    }
}

#[derive(Default)]
pub struct PreviousViewUniforms {
    pub uniforms: DynamicUniformVec<ViewUniform>,
}

#[derive(Clone, Default)]
pub struct PreviousViewUniformOffset {
    pub offset: u32,
}

#[derive(Default)]
pub struct PreviousViewUniformOffsets {
    pub offsets: HashMap<Entity, PreviousViewUniformOffset>,
}

fn save_view_uniform(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    view_uniforms: Res<ViewUniforms>,
    mut previous: ResMut<PreviousViewUniforms>,
    mut previous_offsets: ResMut<PreviousViewUniformOffsets>,
    query: Query<(Entity, &ViewUniformOffset)>,
) {
    previous
        .uniforms
        .reserve_and_clear(view_uniforms.uniforms.len(), &*device);

    for (view_uniform) in view_uniforms.uniforms.iter() {
        previous.uniforms.push(view_uniform.clone());
    }
    previous.uniforms.write_buffer(&*queue);

    previous_offsets.offsets.clear();
    for (entity, offset) in query.iter() {
        previous_offsets.offsets.insert(
            entity,
            PreviousViewUniformOffset {
                offset: offset.offset,
            },
        );
    }
}

fn add_previous_view_offsets(
    mut commands: Commands,
    previous_offsets: Res<PreviousViewUniformOffsets>,
) {
    println!("Adding view offsets");
    let insertable: Vec<_> = previous_offsets
        .offsets
        .iter()
        .map(|(entity, offset)| (*entity, (offset.clone(),)))
        .collect();

    commands.insert_or_spawn_batch(insertable);
}

pub struct TaaMeshBindGroup {
    pub value: BindGroup,
}

fn queue_taa_mesh_bind_group(
    mut commands: Commands,
    taa_shader: Res<TaaVelocityShaders>,
    render_device: Res<RenderDevice>,
    mesh_uniforms: Res<ComponentUniforms<MeshUniform>>,
    previous_mesh_uniforms_option: Option<Res<ComponentUniforms<Previous<MeshUniform>>>>,
) {
    println!("Queueing meshes");
    if previous_mesh_uniforms_option.is_none() { return; }
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

fn queue_taa_view_bind_group(
    mut commands: Commands,
    taa_shader: Res<TaaVelocityShaders>,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
    previous_view_uniforms_option: Option<Res<PreviousViewUniforms>>,
) {
    println!("Queueing views");

    if previous_view_uniforms_option.is_none() {
        return;
    }
    let previous_view_uniforms = previous_view_uniforms_option.unwrap();

    if let (Some(binding), Some(binding2)) = (
        view_uniforms.uniforms.binding(),
        previous_view_uniforms.uniforms.binding(),
    ) {
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

fn queue_velocity(
    velocity_draw_functions: Res<DrawFunctions<Velocity>>,
    meshes: Query<(Entity, &MeshUniform), (With<Handle<Mesh>>, With<Previous<MeshUniform>>)>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Velocity>)>,
) {
    let draw_velocity = velocity_draw_functions
        .read()
        .get_id::<DrawTaaVelocity>()
        .unwrap();

    for (view, mut velocity_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_uniform) in meshes.iter() {
            velocity_phase.add(Velocity {
                entity,
                distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                draw_velocity,
            });
        }
    }
    println!("Queued velocities");
}

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
    fn new(world: &World) -> Self {
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
        println!("Running velocity texture node");
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

struct Velocity {
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

type DrawTaaVelocity = (
    SetVelocityPipeline,
    SetTaaMeshBindGroup<0>,
    SetTaaViewBindGroup<1>,
    DrawMesh,
);

struct SetVelocityPipeline;

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

struct SetTaaMeshBindGroup<const I: usize>;

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

struct SetTaaViewBindGroup<const I: usize>;

impl<const I: usize> RenderCommand<Velocity> for SetTaaViewBindGroup<I> {
    type Param = (
        SQuery<(
            Read<ViewUniformOffset>,
            Read<PreviousViewUniformOffset>,
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
            &[view_index.offset, previous_view_index.offset],
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

struct TaaVelocityNode {
    query: QueryState<Read<RenderPhase<Velocity>>>,
}

impl TaaVelocityNode {
    pub const VIEW_ENTITY: &'static str = "view_entity";
    pub const TARGET: &'static str = "view_texture";
    fn new(world: &mut World) -> Self {
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
        println!("Running velocity node");
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




struct Movable;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // ground plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0 })),
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            ..Default::default()
        }),
        ..Default::default()
    });

    // left wall
    let mut transform = Transform::from_xyz(2.5, 2.5, 0.0);
    transform.rotate(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(5.0, 0.15, 5.0))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::INDIGO,
            perceptual_roughness: 1.0,
            ..Default::default()
        }),
        ..Default::default()
    });
    // back (right) wall
    let mut transform = Transform::from_xyz(0.0, 2.5, -2.5);
    transform.rotate(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2));
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Box::new(5.0, 0.15, 5.0))),
        transform,
        material: materials.add(StandardMaterial {
            base_color: Color::INDIGO,
            perceptual_roughness: 1.0,
            ..Default::default()
        }),
        ..Default::default()
    });

    // cube
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(StandardMaterial {
                base_color: Color::PINK,
                ..Default::default()
            }),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..Default::default()
        })
        .insert(Movable);
    // sphere
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.5,
                ..Default::default()
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::LIME_GREEN,
                ..Default::default()
            }),
            transform: Transform::from_xyz(1.5, 1.0, 1.5),
            ..Default::default()
        })
        .insert(Movable);

    // ambient light
    commands.insert_resource(AmbientLight {
        color: Color::ORANGE_RED,
        brightness: 0.02,
    });

    // red point light
    commands
        .spawn_bundle(PointLightBundle {
            // transform: Transform::from_xyz(5.0, 8.0, 2.0),
            transform: Transform::from_xyz(1.0, 2.0, 0.0),
            point_light: PointLight {
                intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                color: Color::RED,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1,
                    ..Default::default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::RED,
                    emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

    // green point light
    commands
        .spawn_bundle(PointLightBundle {
            // transform: Transform::from_xyz(5.0, 8.0, 2.0),
            transform: Transform::from_xyz(-1.0, 2.0, 0.0),
            point_light: PointLight {
                intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                color: Color::GREEN,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1,
                    ..Default::default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::GREEN,
                    emissive: Color::rgba_linear(0.0, 100.0, 0.0, 0.0),
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

    // blue point light
    commands
        .spawn_bundle(PointLightBundle {
            // transform: Transform::from_xyz(5.0, 8.0, 2.0),
            transform: Transform::from_xyz(0.0, 4.0, 0.0),
            point_light: PointLight {
                intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                color: Color::BLUE,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1,
                    ..Default::default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::BLUE,
                    emissive: Color::rgba_linear(0.0, 0.0, 100.0, 0.0),
                    ..Default::default()
                }),
                ..Default::default()
            });
        });

    // directional 'sun' light
    const HALF_SIZE: f32 = 10.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            // Configure the projection to better fit the scene
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..Default::default()
            },
            ..Default::default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..Default::default()
        },
        ..Default::default()
    });

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    println!("Setup done");
}

// fn animate_light_direction(
//     time: Res<Time>,
//     mut query: Query<&mut Transform, With<DirectionalLight>>,
// ) {
//     for mut transform in query.iter_mut() {
//         transform.rotate(Quat::from_rotation_y(time.delta_seconds() * 0.5));
//     }
// }

fn movement(
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Movable>>,
) {
    for mut transform in query.iter_mut() {
        let t = time.seconds_since_startup();
        let direction = Vec3::new(t.cos() as f32, 0.0, t.sin() as f32);


        transform.translation += time.delta_seconds() * 2.0 * direction;
    }
}


pub fn extract_velocity_camera_phases(
    mut commands: Commands,
    active_cameras: Res<ActiveCameras>,
) {
    
    if let Some(camera_3d) = active_cameras.get(CameraPlugin::CAMERA_3D) {
        if let Some(entity) = camera_3d.entity {
            commands
                .get_or_spawn(entity)
                .insert(RenderPhase::<Velocity>::default());
        }
    }
}

fn main() {
    App::new()
        .add_plugins(PipelinedDefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(TaaPlugin)
        .add_startup_system(setup)
        .add_system(movement)
        // .add_system(animate_light_direction)
        .run();
}
