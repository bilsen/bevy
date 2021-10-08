use crate::{
    render_graph::{SlotType, SlotValue},
    render_resource::TextureView,
};
use bevy_ecs::entity::Entity;
use bevy_utils::HashMap;
use std::borrow::Cow;

use super::{RenderGraphId, SlotInfo};

pub struct RunSubGraph {
    pub id: RenderGraphId,
    pub context: GraphContext,
}

#[derive(Default)]
pub struct RunSubGraphs {
    commands: Vec<RunSubGraph>,
}

impl RunSubGraphs {
    pub fn run(&mut self, id: RenderGraphId, inputs: impl Into<GraphContext>) {
        self.commands.push(RunSubGraph {
            id,
            context: inputs.into(),
        });
    }

    pub fn iter(&self) -> impl Iterator<Item = &RunSubGraph> {
        self.commands.iter()
    }

    pub fn drain(self) -> impl Iterator<Item = RunSubGraph> {
        self.commands.into_iter()
    }
}

#[derive(Clone, Default)]
pub struct GraphContext {
    inputs: HashMap<SlotInfo, SlotValue>,
}

impl GraphContext {
    pub fn new(inputs: HashMap<SlotInfo, SlotValue>) -> Self {
        Self { inputs }
    }

    pub fn get_input_entity(&self, name: impl Into<&'static str>) -> &Entity {
        if let SlotValue::Entity(entity) = self
            .inputs
            .get(&SlotInfo::new(name.into(), SlotType::Entity))
            .expect("No input value found")
        {
            return entity;
        } else {
            panic!("Wrong input type")
        }
    }

    pub fn get_input_texture(&self, name: impl Into<&'static str>) -> &TextureView {
        if let SlotValue::TextureView(texture_view) = self
            .inputs
            .get(&SlotInfo::new(name.into(), SlotType::TextureView))
            .expect("No input value found")
        {
            return texture_view;
        } else {
            panic!("Wrong input type")
        }
    }
}

impl<I: Into<Cow<'static, str>>, T: IntoIterator<Item = (I, SlotValue)>> From<T> for GraphContext {
    fn from(iterator: T) -> Self {
        Self {
            inputs: iterator
                .into_iter()
                .map(|(label, value)| (SlotInfo::new(label.into(), value.slot_type()), value))
                .collect(),
        }
    }
}

// #[derive(Error, Debug, Eq, PartialEq)]
// pub enum RunSubGraphError {
//     #[error("tried to run a non-existent sub-graph")]
//     MissingSubGraph(Cow<'static, str>),
//     #[error("passed in inputs, but this sub-graph doesn't have any")]
//     SubGraphHasNoInputs(Cow<'static, str>),
//     #[error("sub graph (name: '{graph_name:?}') could not be run because slot '{slot_name}' at index {slot_index} has no value")]
//     MissingInput {
//         slot_index: usize,
//         slot_name: Cow<'static, str>,
//         graph_name: Cow<'static, str>,
//     },
//     #[error("attempted to use the wrong type for input slot")]
//     MismatchedInputSlotType {
//         graph_name: Cow<'static, str>,
//         slot_index: usize,
//         label: SlotLabel,
//         expected: SlotType,
//         actual: SlotType,
//     },
// }
