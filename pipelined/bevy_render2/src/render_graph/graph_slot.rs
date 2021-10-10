use bevy_ecs::entity::Entity;
use bevy_utils::HashMap;
use std::borrow::Cow;

use crate::render_resource::{Buffer, Sampler, TextureView};

use super::GraphContext;

#[derive(Debug, Clone)]
pub enum SlotValue {
    Buffer(Buffer),
    TextureView(TextureView),
    Sampler(Sampler),
    Entity(Entity),
}

impl SlotValue {
    pub fn slot_type(&self) -> SlotType {
        match self {
            SlotValue::Buffer(_) => SlotType::Buffer,
            SlotValue::TextureView(_) => SlotType::TextureView,
            SlotValue::Sampler(_) => SlotType::Sampler,
            SlotValue::Entity(_) => SlotType::Entity,
        }
    }
}

impl From<Buffer> for SlotValue {
    fn from(value: Buffer) -> Self {
        SlotValue::Buffer(value)
    }
}

impl From<TextureView> for SlotValue {
    fn from(value: TextureView) -> Self {
        SlotValue::TextureView(value)
    }
}

impl From<Sampler> for SlotValue {
    fn from(value: Sampler) -> Self {
        SlotValue::Sampler(value)
    }
}

impl From<Entity> for SlotValue {
    fn from(value: Entity) -> Self {
        SlotValue::Entity(value)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SlotType {
    Buffer,
    TextureView,
    Sampler,
    Entity,
}

impl SlotType {
    pub fn as_string(&self) -> String {
        match self {
            SlotType::Buffer => "Buffer",
            SlotType::Entity => "Entity",
            SlotType::Sampler => "Sampler",
            SlotType::TextureView => "TextureView",
        }
        .into()
    }
}

#[derive(Default, Debug)]
pub struct SlotInfos {
    slots: HashMap<Cow<'static, str>, SlotType>,
}

impl<N: Into<Cow<'static, str>>, T: IntoIterator<Item = (N, SlotType)>> From<T> for SlotInfos {
    fn from(slots: T) -> Self {
        SlotInfos {
            slots: slots
                .into_iter()
                .map(|(name, slot_type)| (name.into(), slot_type))
                .collect(),
        }
    }
}

impl SlotInfos {
    #[inline]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn get_slot_type(&self, name: impl Into<Cow<'static, str>>) -> Option<&SlotType> {
        self.slots.get(&name.into())
    }
    pub fn add_slot(&mut self, name: impl Into<Cow<'static, str>>, slot_type: SlotType) {
        self.slots.insert(name.into(), slot_type);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Cow<'static, str>, &SlotType)> {
        self.slots.iter()
    }

    pub fn matches(&self, context: &GraphContext) -> bool {
        for (name, value) in context.iter() {
            if let Some(slot_type) = self.slots.get(name) {
                if slot_type != &value.slot_type() {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    pub fn as_string(&self) -> String {
        let type_strings: Vec<_> = self
            .slots
            .iter()
            .map(|(name, slot_type)| (format!("{}: {}", name.to_owned(), slot_type.as_string())))
            .collect();
        type_strings.join(", ")
    }
}
