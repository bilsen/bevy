use std::{array::IntoIter, cell::UnsafeCell, collections::hash_map::Iter, fmt::Debug, iter::Map, marker::PhantomData, ops::Deref, sync::Arc};

use bevy_utils::HashMap;

use crate::{change_detection::Ticks, component::{Component, ComponentId, ComponentTicks}, prelude::*};

use super::{SystemId, SystemMeta, SystemParam, SystemParamFetch, SystemParamState};


/// A resource which is coupled to this system
pub struct SystemRes<'w, T: Component> {
    value: &'w T,
    ticks: &'w ComponentTicks,
    last_change_tick: u32,
    change_tick: u32,
}

impl<'a, T: Component + Default> SystemParam for SystemRes<'a, T> {
    type Fetch = SystemResState<T>;
}

impl<'a, T: Component + Default> SystemParam for SystemResMut<'a, T> {
    type Fetch = SystemResMutState<T>;
}

impl<'a, T: Component + Default> SystemParam for AllSystemRes<'a, T> {
    type Fetch = AllSystemResState<T>;
}

impl<'a, T: Component + Default> SystemParam for AllSystemResMut<'a, T> {
    type Fetch = AllSystemResMutState<T>;
}

// impl<'a, T: Component + Default> SystemParam for SystemsResMut<'a, T> {
//     type Fetch = SystemsResMutState<T>;
// }




impl<'w, T: Component> Debug for SystemRes<'w, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Res").field(&self.value).finish()
    }
}

impl<'w, T: Component> SystemRes<'w, T> {
    /// Returns true if (and only if) this resource been added since the last execution of this
    /// system.
    pub fn is_added(&self) -> bool {
        self.ticks.is_added(self.last_change_tick, self.change_tick)
    }

    /// Returns true if (and only if) this resource been changed since the last execution of this
    /// system.
    pub fn is_changed(&self) -> bool {
        self.ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }

    pub fn into_inner(self) -> &'w T {
        self.value
    }
}

impl<'w, T: Component> Deref for SystemRes<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T: Component> AsRef<T> for SystemRes<'w, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

/// The collection of all SystemRes<T> (immutable)


/// SAFE
unsafe impl<T> Sync for AllSystemResInternal<T> {}


struct AllSystemResInternal<T> {
    hash_map: HashMap<SystemId, UnsafeCell<(T, ComponentTicks)>>,
}

impl<T: Component> AllSystemResInternal<T> {


    unsafe fn get_system_res_unchecked(&self, id: SystemId, last_change_tick: u32, change_tick: u32) -> SystemRes<'_, T> {
        let (val, ticks) = &*self.hash_map.get(&id).unwrap().get();
        Self::make_system_res(val, ticks, last_change_tick, change_tick)
    }
    
    unsafe fn get_system_res_mut_unchecked(&self, id: SystemId, last_change_tick: u32, change_tick: u32) -> SystemResMut<'_, T> {
        let (val, ticks) = &mut *self.hash_map.get(&id).unwrap().get();
        SystemResMut {
            value: val,
            ticks: Ticks {
                component_ticks: ticks,
                last_change_tick,
                change_tick
            }
        }
    }

    fn make_system_res<'w>(value: &'w T, ticks: &'w ComponentTicks, last_change_tick: u32, change_tick: u32) -> SystemRes<'w, T> {
        SystemRes {
            value,
            ticks,
            last_change_tick,
            change_tick
        }
    }

    fn make_system_res_mut<'w>(value: &'w mut T, ticks: &'w mut ComponentTicks, last_change_tick: u32, change_tick: u32) -> SystemResMut<'w, T> {
        SystemResMut {
            value,
            ticks: Ticks {
                component_ticks: ticks,
                last_change_tick,
                change_tick
            }
        }
    }

    fn insert(&mut self, id: SystemId, val: T) {
        self.hash_map.insert(id, UnsafeCell::new((val, ComponentTicks::new(0))));
    }

}

impl<T> Default for AllSystemResInternal<T> {
    fn default() -> Self {
        Self {
            hash_map: Default::default()
        }
    }
}

/// The [`SystemParamState`] of [`Res<T>`].
pub struct SystemResState<T> {
    internal_component_id: ComponentId,
    marker: PhantomData<T>,
}

// SAFE: Res ComponentId and ArchetypeComponentId access is applied to SystemMeta. If this Res
// conflicts with any prior access, a panic will occur.
unsafe impl<T: Component + Default> SystemParamState for SystemResState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
        let internal_component_id = world.components.get_or_insert_resource_id::<AllSystemResInternal<T>>();
        let entire_marker_id = world.components.get_or_insert_resource_id::<EntireMarker<T>>();
        let locally_marker_id = world.components.get_or_insert_resource_id::<LocallyMarker<T>>();

        world.initialize_resource::<EntireMarker<T>>();
        world.initialize_resource::<LocallyMarker<T>>();
        let mut internal = world.get_resource_or_insert_with(<AllSystemResInternal<T> as Default>::default);

        internal.insert(system_meta.id, Default::default());
        let combined_access = system_meta.component_access_set.combined_access_mut();
        
        match (
            combined_access.has_read(locally_marker_id),
            combined_access.has_write(locally_marker_id),
            combined_access.has_read(entire_marker_id),
            combined_access.has_write(entire_marker_id)
        ) {
            // Only case which is fine (no other *SystemResMut<T> on this system)
            (_, false, _, false) => {},
            // TODO: Add constructive errors
            _ => {
                panic!("parameter error")
            }
        }

        // Set the EntireMarker to read
        {
            combined_access.add_read(entire_marker_id);
    
            let resource_archetype = world.archetypes.resource();
            let entire_archetype_component_id = resource_archetype
                .get_archetype_component_id(entire_marker_id)
                .unwrap();
                
            system_meta
                .archetype_component_access
                .add_read(entire_archetype_component_id);
        }

        // Set the LocallyMarker to read
        {
            combined_access.add_read(locally_marker_id);
    
            let resource_archetype = world.archetypes.resource();
            let locally_archetype_component_id = resource_archetype
                .get_archetype_component_id(locally_marker_id)
                .unwrap();
                
            system_meta
                .archetype_component_access
                .add_read(locally_archetype_component_id);
        }

        Self {
            internal_component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: Component + Default> SystemParamFetch<'w, 's> for SystemResState<T> {
    type Item = SystemRes<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let column = world
            .get_populated_resource_column(state.internal_component_id)
            .unwrap_or_else(|| {
                panic!(
                    "System Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        
        let internal = &*column.get_data_ptr().cast::<AllSystemResInternal<T>>().as_ptr();

        internal.get_system_res_unchecked(system_meta.id, system_meta.last_change_tick, change_tick)
    }
}


pub struct SystemResMut<'a, T: Component> {
    pub(crate) value: &'a mut T,
    pub(crate) ticks: Ticks<'a>,
}
use std::ops::DerefMut;

change_detection_impl!(SystemResMut<'a, T>, T, Component);
impl_into_inner!(SystemResMut<'a, T>, T, Component);
impl_debug!(SystemResMut<'a, T>, Component);


pub struct SystemResMutState<T> {
    internal_component_id: ComponentId,
    marker: PhantomData<T>,
}

unsafe impl<T: Component + Default> SystemParamState for SystemResMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
            
        let internal_component_id = world.components.get_or_insert_resource_id::<AllSystemResInternal<T>>();
        let entire_marker_id = world.components.get_or_insert_resource_id::<EntireMarker<T>>();
        let locally_marker_id = world.components.get_or_insert_resource_id::<LocallyMarker<T>>();

        world.get_resource_or_insert_with(<AllSystemResInternal<T> as Default>::default);
        world.initialize_resource::<EntireMarker<T>>();
        world.initialize_resource::<LocallyMarker<T>>();

        let internal = &mut *world.get_resource_or_insert_with(<AllSystemResInternal<T> as Default>::default);
        let combined_access = system_meta.component_access_set.combined_access_mut();


        internal.insert(system_meta.id, Default::default());
        


        
            
        match (
            combined_access.has_read(locally_marker_id),
            combined_access.has_write(locally_marker_id),
            combined_access.has_read(entire_marker_id),
            combined_access.has_write(entire_marker_id)
        ) {
            // Only case which is fine (no other *SystemRes*<T> on this system)
            (false, false, false, false) => {},
            // TODO: Add constructive errors
            _ => {
                panic!("parameter error")
            }
        }

        // Set the EntireMarker to read
        {
            combined_access.add_read(entire_marker_id);
    
            let resource_archetype = world.archetypes.resource();
            let entire_archetype_component_id = resource_archetype
                .get_archetype_component_id(entire_marker_id)
                .unwrap();
                
            system_meta
                .archetype_component_access
                .add_read(entire_archetype_component_id);
        }

        // Set the LocallyMarker to write
        {
            combined_access.add_write(locally_marker_id);
    
            let resource_archetype = world.archetypes.resource();
            let locally_archetype_component_id = resource_archetype
                .get_archetype_component_id(locally_marker_id)
                .unwrap();
                
            system_meta
                .archetype_component_access
                .add_write(locally_archetype_component_id);
        }
        

        Self {
            internal_component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: Component + Default> SystemParamFetch<'w, 's> for SystemResMutState<T> {
    type Item = SystemResMut<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let column = world
            .get_populated_resource_column(state.internal_component_id)
            .unwrap_or_else(|| {
                panic!(
                    "System Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        
        let internal = &*column.get_data_ptr().cast::<AllSystemResInternal<T>>().as_ptr();
        
        internal.get_system_res_mut_unchecked(system_meta.id, system_meta.last_change_tick, change_tick)
    }
}




/// Used for a system to signal that it has complete read/write access to AllSystemRes<T>
#[derive(Default)]
struct EntireMarker<T: Component>(PhantomData<T>);

/// Used for a system to signal that it has read/write access to AllSystemRes<T>
#[derive(Default)]
struct LocallyMarker<T: Component>(PhantomData<T>);

// |____________________| EntireMarker<T> | LocallyMarker<T>
// | AllSystemRes<T>    | read            | read
// | AllSystemResMut<T> | write           | write
// | SystemRes<T>       | read            | read
// | SystemResMut<T>    | read            | write
//
// The LocallyMarker<T> is how this system interacts with its own System Resource.
// The EntireMarker<T> is how this system interacts with all the System Resources.
//

pub struct AllSystemRes<'w, T> {
    internal: &'w AllSystemResInternal<T>,
    ticks: &'w ComponentTicks,
    last_change_tick: u32,
    change_tick: u32,
}

pub struct AllSystemResState<T> {
    internal_component_id: ComponentId,
    marker: PhantomData<T>,
}



unsafe impl<T: Component + Default> SystemParamState for AllSystemResState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
            
        let internal_component_id = world.components.get_or_insert_resource_id::<AllSystemResInternal<T>>();
        let entire_marker_id = world.components.get_or_insert_resource_id::<EntireMarker<T>>();
        let locally_marker_id = world.components.get_or_insert_resource_id::<LocallyMarker<T>>();

        let internal = &mut *world.get_resource_or_insert_with(<AllSystemResInternal<T> as Default>::default);
        world.initialize_resource::<EntireMarker<T>>();
        world.initialize_resource::<LocallyMarker<T>>();

        let combined_access = system_meta.component_access_set.combined_access_mut();



        match (
            combined_access.has_read(locally_marker_id),
            combined_access.has_write(locally_marker_id),
            combined_access.has_read(entire_marker_id),
            combined_access.has_write(entire_marker_id)
        ) {
            // Only case which is fine (no other *SystemResMut<T> on this system)
            (_, false, _, false) => {},
            // TODO: Add constructive errors
            _ => {
                panic!("parameter error")
            }
        }

        // Set the EntireMarker to read
        {
            combined_access.add_read(entire_marker_id);
    
            let resource_archetype = world.archetypes.resource();
            let entire_archetype_component_id = resource_archetype
                .get_archetype_component_id(entire_marker_id)
                .unwrap();
                
            system_meta
                .archetype_component_access
                .add_read(entire_archetype_component_id);
        }

        // Set the LocallyMarker to read
        {
            combined_access.add_read(locally_marker_id);
    
            let resource_archetype = world.archetypes.resource();
            let locally_archetype_component_id = resource_archetype
                .get_archetype_component_id(locally_marker_id)
                .unwrap();
                
            system_meta
                .archetype_component_access
                .add_read(locally_archetype_component_id);
        }
        
        Self {
            internal_component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: Component + Default> SystemParamFetch<'w, 's> for AllSystemResState<T> {

    type Item = AllSystemRes<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let column = world
            .get_populated_resource_column(state.internal_component_id)
            .unwrap_or_else(|| {
                panic!(
                    "System Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        
        let internal = &*column.get_data_ptr().cast::<AllSystemResInternal<T>>().as_ptr();

        AllSystemRes {
            internal: internal,
            ticks: column.get_ticks_unchecked(0),
            last_change_tick: system_meta.last_change_tick,
            change_tick,
        }
    }
}

impl<'w, T: Component> AllSystemRes<'w, T> {

    pub fn iter(&self) -> impl Iterator<Item = (SystemId, SystemRes<'w, T>)>
    {
        let last_change_tick = self.last_change_tick;
        let change_tick = self.change_tick;
        self.internal.hash_map.iter().map(move |(id, cell)| {
            (
                *id, 
                {
                    let (val, ticks) = unsafe {
                        &*cell.get()
                    };
                    AllSystemResInternal::<T>::make_system_res(val, ticks, last_change_tick, change_tick)
                }
            )
        })
    }
}


pub struct AllSystemResMut<'w, T> {
    internal: &'w AllSystemResInternal<T>,
    ticks: &'w ComponentTicks,
    last_change_tick: u32,
    change_tick: u32,
}

pub struct AllSystemResMutState<T> {
    internal_component_id: ComponentId,
    marker: PhantomData<T>,
}



unsafe impl<T: Component + Default> SystemParamState for AllSystemResMutState<T> {
    type Config = ();

    fn init(world: &mut World, system_meta: &mut SystemMeta, _config: Self::Config) -> Self {
            
        let internal_component_id = world.components.get_or_insert_resource_id::<AllSystemResInternal<T>>();
        let entire_marker_id = world.components.get_or_insert_resource_id::<EntireMarker<T>>();
        let locally_marker_id = world.components.get_or_insert_resource_id::<LocallyMarker<T>>();

        world.get_resource_or_insert_with(<AllSystemResInternal<T> as Default>::default);
        world.initialize_resource::<EntireMarker<T>>();
        world.initialize_resource::<LocallyMarker<T>>();



        let combined_access = system_meta.component_access_set.combined_access_mut();



        match (
            combined_access.has_read(locally_marker_id),
            combined_access.has_write(locally_marker_id),
            combined_access.has_read(entire_marker_id),
            combined_access.has_write(entire_marker_id)
        ) {
            // Only case which is fine (no other *SystemRes*<T> on this system)
            (false, false, false, false) => {},
            // TODO: Add constructive errors
            _ => {
                panic!("parameter error")
            }
        }

        // Set the EntireMarker to write
        {
            combined_access.add_write(entire_marker_id);
    
            let resource_archetype = world.archetypes.resource();
            let entire_archetype_component_id = resource_archetype
                .get_archetype_component_id(entire_marker_id)
                .unwrap();
                
            system_meta
                .archetype_component_access
                .add_write(entire_archetype_component_id);
        }

        // Set the LocallyMarker to write
        {
            combined_access.add_write(locally_marker_id);
    
            let resource_archetype = world.archetypes.resource();
            let locally_archetype_component_id = resource_archetype
                .get_archetype_component_id(locally_marker_id)
                .unwrap();
                
            system_meta
                .archetype_component_access
                .add_write(locally_archetype_component_id);
        }
        
        Self {
            internal_component_id,
            marker: PhantomData,
        }
    }

    fn default_config() {}
}

impl<'w, 's, T: Component + Default> SystemParamFetch<'w, 's> for AllSystemResMutState<T> {

    type Item = AllSystemResMut<'w, T>;

    #[inline]
    unsafe fn get_param(
        state: &'s mut Self,
        system_meta: &SystemMeta,
        world: &'w World,
        change_tick: u32,
    ) -> Self::Item {
        let column = world
            .get_populated_resource_column(state.internal_component_id)
            .unwrap_or_else(|| {
                panic!(
                    "System Resource requested by {} does not exist: {}",
                    system_meta.name,
                    std::any::type_name::<T>()
                )
            });
        
        let internal = &*column.get_data_ptr().cast::<AllSystemResInternal<T>>().as_ptr();
        

        AllSystemResMut {
            internal: internal,
            ticks: column.get_ticks_unchecked(0),
            last_change_tick: system_meta.last_change_tick,
            change_tick,
        }
    }
}

impl<'w, T: Component> AllSystemResMut<'w, T> {

    pub fn iter(&self) -> impl Iterator<Item = (SystemId, SystemRes<'w, T>)>
    {
        let last_change_tick = self.last_change_tick;
        let change_tick = self.change_tick;
        self.internal.hash_map.iter().map(move |(id, cell)| {
            (
                *id, 
                {
                    let (val, ticks) = unsafe {
                        &*cell.get()
                    };
                    AllSystemResInternal::<T>::make_system_res(val, ticks, last_change_tick, change_tick)
                }
            )
        })
    }

    pub fn iter_mut(&self) -> impl Iterator<Item = (SystemId, SystemResMut<'w, T>)>
    {
        let last_change_tick = self.last_change_tick;
        let change_tick = self.change_tick;
        self.internal.hash_map.iter().map(move |(id, cell)| {
            (
                *id, 
                {
                    let (val, ticks) = unsafe {
                        &mut *cell.get()
                    };
                    AllSystemResInternal::<T>::make_system_res_mut(val, ticks, last_change_tick, change_tick)
                }
            )
        })
    }
}
