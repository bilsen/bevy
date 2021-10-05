use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    schedule::ExclusiveSystemDescriptorCoercion,
    system::{Commands, IntoExclusiveSystem, Query, ResMut},
};
use bevy_render2::{RenderApp, RenderStage};
use bevy_utils::HashMap;
use crevice::std140::AsStd140;

pub struct SaveComponentPlugin<C: Component + Clone> {
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
pub struct Previous<C: Component>(pub C);

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
    values: Vec<(Entity, C)>,
}

impl<C: Component> Default for Saved<C> {
    fn default() -> Self {
        Self {
            values: Vec::default(),
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
        saved_resource.values.push((entity, component.clone()));
    }
}

fn add_saved_components_system<C: Component>(
    mut commands: Commands,
    mut saved_resouce: ResMut<Saved<C>>,
) {
    let bundles_iter: Vec<(Entity, (Previous<C>,))> = saved_resouce
        .values
        .drain(..)
        .map(|(entity, component)| (entity, (Previous(component),)))
        .collect();

    commands.insert_or_spawn_batch(bundles_iter);
}
