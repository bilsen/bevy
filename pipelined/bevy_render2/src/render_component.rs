use crate::{
    render_resource::DynamicUniformVec,
    renderer::{RenderDevice, RenderQueue},
    RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_asset::{Asset, Handle};
use bevy_ecs::{
    component::Component,
    prelude::*,
    query::{FilterFetch, QueryItem, ReadOnlyFetch, WorldQuery},
    system::{
        lifetimeless::{Read, SCommands, SQuery},
        RunSystem, SystemParamItem,
    },
};
use crevice::std140::AsStd140;
use std::{marker::PhantomData, ops::Deref};

pub struct DynamicUniformIndex<C: Component> {
    index: u32,
    marker: PhantomData<C>,
}

impl<C: Component> DynamicUniformIndex<C> {
    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }
}

pub trait ExtractComponent: Component {
    type Query: WorldQuery;
    type Filter: WorldQuery;
    fn extract_component(item: QueryItem<Self::Query>) -> Self;
}

/// Extracts assets into gpu-usable data
pub struct UniformComponentPlugin<C, U = C> {
    func: fn(&C) -> U,
    _marker: PhantomData<fn() -> (C, U)>,
}

fn id_clone<C: Clone>(input: &C) -> C {
    input.clone()
}

impl<C: Clone> Default for UniformComponentPlugin<C, C> {
    fn default() -> Self {
        Self {
            func: id_clone,
            _marker: PhantomData,
        }
    }
}

impl<C: Component, U: AsStd140 + Send + Sync + 'static> UniformComponentPlugin<C, U> {
    pub fn new(func: fn(&C) -> U) -> Self {
        Self {
            func,
            _marker: PhantomData,
        }
    }
}

impl<C: Component, U: AsStd140 + Send + Sync + 'static> Plugin for UniformComponentPlugin<C, U> {
    fn build(&self, app: &mut App) {
        let func = self.func.clone();

        app.sub_app(RenderApp)
            .insert_resource(ComponentUniforms::<U>::default())
            .add_system_to_stage(
                RenderStage::Prepare,
                move |mut commands: Commands,
                      render_device: Res<RenderDevice>,
                      render_queue: Res<RenderQueue>,
                      mut component_uniforms: ResMut<ComponentUniforms<U>>,
                      components: Query<(Entity, &C)>| {
                    prepare_uniform_components(
                        commands,
                        render_device,
                        render_queue,
                        component_uniforms,
                        components,
                        func,
                    );
                },
            );
    }
}

pub struct ComponentUniforms<C: AsStd140> {
    uniforms: DynamicUniformVec<C>,
}

impl<C: Component + AsStd140> Deref for ComponentUniforms<C> {
    type Target = DynamicUniformVec<C>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.uniforms
    }
}

impl<C: Component + AsStd140> ComponentUniforms<C> {
    #[inline]
    pub fn uniforms(&self) -> &DynamicUniformVec<C> {
        &self.uniforms
    }
}

impl<C: Component + AsStd140> Default for ComponentUniforms<C> {
    fn default() -> Self {
        Self {
            uniforms: Default::default(),
        }
    }
}

fn prepare_uniform_components<C: Component, U: AsStd140 + Send + Sync + 'static>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut component_uniforms: ResMut<ComponentUniforms<U>>,
    components: Query<(Entity, &C)>,
    mapping: fn(&C) -> U,
) {
    let len = components.iter().len();
    component_uniforms
        .uniforms
        .reserve_and_clear(len, &render_device);
    for (entity, component) in components.iter() {
        commands
            .get_or_spawn(entity)
            .insert(DynamicUniformIndex::<U> {
                index: component_uniforms.uniforms.push(mapping(component)),
                marker: PhantomData,
            });
    }

    component_uniforms.uniforms.write_buffer(&render_queue);
}

pub struct ExtractComponentPlugin<C, F = ()>(PhantomData<fn() -> (C, F)>);

impl<C, F> Default for ExtractComponentPlugin<C, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C: ExtractComponent> Plugin for ExtractComponentPlugin<C>
where
    <C::Query as WorldQuery>::Fetch: ReadOnlyFetch,
    <C::Filter as WorldQuery>::Fetch: FilterFetch,
{
    fn build(&self, app: &mut App) {
        let system = ExtractComponentSystem::<C>::system(&mut app.world);
        let render_app = app.sub_app(RenderApp);
        render_app.add_system_to_stage(RenderStage::Extract, system);
    }
}

impl<T: Asset> ExtractComponent for Handle<T> {
    type Query = Read<Handle<T>>;
    type Filter = ();

    #[inline]
    fn extract_component(handle: QueryItem<Self::Query>) -> Self {
        handle.clone_weak()
    }
}

pub struct ExtractComponentSystem<C: ExtractComponent>(PhantomData<C>);

impl<C: ExtractComponent> RunSystem for ExtractComponentSystem<C>
where
    <C::Filter as WorldQuery>::Fetch: FilterFetch,
    <C::Query as WorldQuery>::Fetch: ReadOnlyFetch,
{
    type Param = (
        SCommands,
        Local<'static, usize>,
        SQuery<(Entity, C::Query), C::Filter>,
    );

    fn run((mut commands, mut previous_len, query): SystemParamItem<Self::Param>) {
        let mut values = Vec::with_capacity(*previous_len);
        for (entity, query_item) in query.iter() {
            values.push((entity, (C::extract_component(query_item),)));
        }
        *previous_len = values.len();
        commands.insert_or_spawn_batch(values);
    }
}
