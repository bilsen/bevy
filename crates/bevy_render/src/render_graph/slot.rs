use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    fmt::Debug,
};

use bevy_ecs::entity::Entity;
use bevy_utils::{HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};

use super::{SlotError, SlotRequirementError, SlotValueError};

pub trait SlotValue: Downcast + Send + Sync + 'static {}

impl_downcast!(SlotValue);

impl SlotValue for Entity {}

type BoxedSlotValue = Box<dyn SlotValue>;

#[derive(Default)]
pub struct SlotRequirements {
    values: HashMap<Cow<'static, str>, TypeId>,
    type_names: HashMap<TypeId, &'static str>,
    defaults: HashMap<Cow<'static, str>, BoxedSlotValue>,
}

impl SlotRequirements {
    pub fn merge(&mut self, other: Self) -> Result<(), SlotError> {
        for (key, new_type) in other.values.into_iter() {
            if let Some(current_type) = self.values.get(&key) {
                if current_type != &new_type {
                    return Err(SlotError::SlotConflict {
                        slot_name: key,
                        current_type: self.type_names[current_type].clone().into(),
                        new_type: other.type_names[&new_type].clone().into(),
                    });
                }
            }

            self.type_names
                .insert(new_type, other.type_names[&new_type].clone());
            self.values.insert(key, new_type);
        }

        Ok(())
    }

    pub fn with<T: SlotValue>(mut self, into_slot_name: Cow<'static, str>) -> Self {
        let id = TypeId::of::<T>();
        let slot_name = into_slot_name.into();

        let name = type_name::<T>();

        self.type_names.insert(id, name.into());
        self.values.insert(slot_name, id);

        self
    }

    pub fn set_default<T: SlotValue>(
        &mut self,
        slot_name: &Cow<'static, str>,
        value: T,
    ) -> Result<&mut Self, SlotError> {
        if let Some(ty) = self.values.get(slot_name) {
            if ty != &TypeId::of::<T>() {
                let expected_type_name = self.type_names[ty].clone();

                return Err(SlotError::SlotTypeError {
                    slot_name: slot_name.clone(),
                    expected_type: expected_type_name.into(),
                    actual_type: type_name::<T>().into(),
                });
            }
        }

        self.values.insert(slot_name.clone(), TypeId::of::<T>());
        self.defaults.insert(slot_name.clone(), Box::new(value));

        Ok(self)
    }

    pub fn get_slot_requirement_error(
        &self,
        graph_requirements: &SlotRequirements,
    ) -> Option<SlotRequirementError> {
        for (name, id) in self.values.iter() {
            if let Some(expected_id) = graph_requirements.values.get(name) {
                if expected_id != id {
                    let actual = graph_requirements.type_names[id].into();
                    let provided = self.type_names[expected_id].clone().into();
                    return Some(SlotRequirementError::SlotRequirementTypeError {
                        name: name.clone(),
                        actual,
                        provided,
                    });
                }
            } else {
                return Some(SlotRequirementError::SlotRequirementDoesntExist {
                    name: name.clone(),
                });
            }
        }
        None
    }

    /// Returns the first error (if any) for these slot values
    pub fn get_slot_value_error(&self, provided_values: &SlotValues) -> Option<SlotValueError> {
        let mut not_seen: HashSet<_> = self.values.keys().collect();

        for (
            name,
            SlotValueDescriptor {
                type_name: provided,
                type_id,
                ..
            },
        ) in provided_values.values.iter()
        {
            if let Some(expected_id) = self.values.get(name) {
                if expected_id != type_id {
                    return Some(SlotValueError::SlotProvidedTypeError {
                        name: name.clone(),
                        actual: self.type_names[expected_id].clone().into(),
                        provided: provided.clone().into(),
                    });
                }
                not_seen.take(name);
            } else {
                return Some(SlotValueError::SlotDoesntExist { name: name.clone() });
            }
        }
        for not_specified in not_seen.drain() {
            if !self.defaults.contains_key(not_specified) {
                return Some(SlotValueError::SlotNotProvided {
                    name: not_specified.clone(),
                });
            }
        }
        None
    }
}

pub struct SlotValueDescriptor {
    item: Box<dyn SlotValue>,
    type_name: &'static str,
    type_id: TypeId,
}

impl Debug for SlotValueDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotValueDescriptor")
            .field("type_name", &self.type_name)
            .field("type_id", &self.type_id)
            .finish()
    }
}

#[derive(Default, Debug)]
pub struct SlotValues {
    values: HashMap<Cow<'static, str>, SlotValueDescriptor>,
}

impl SlotValues {
    pub fn with<T: SlotValue>(
        mut self,
        into_slot_name: impl Into<Cow<'static, str>>,
        value: T,
    ) -> Self {
        let slot_name = into_slot_name.into();
        self.values.insert(
            slot_name,
            SlotValueDescriptor {
                item: Box::new(value),
                type_name: type_name::<T>(),
                type_id: TypeId::of::<T>(),
            },
        );

        self
    }

    pub fn get<T: SlotValue>(&self, slot_name: &Cow<'static, str>) -> Result<&T, SlotError> {
        let SlotValueDescriptor {
            item,
            type_name: actual_type,
            ..
        } = self
            .values
            .get(slot_name)
            .ok_or_else(|| SlotError::SlotNameError(slot_name.clone()))?;

        item.downcast_ref::<T>().ok_or_else(|| {
            let expected_type = type_name::<T>().into();

            SlotError::SlotTypeError {
                slot_name: slot_name.clone(),
                expected_type,
                actual_type: actual_type.clone().into(),
            }
        })
    }
}
