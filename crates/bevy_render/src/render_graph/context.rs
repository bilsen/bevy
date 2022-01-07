use crate::{
    render_graph::{NodeState, RenderGraph, SlotInfos, SlotLabel, SlotType, SlotValue},
    render_resource::{Buffer, Sampler, TextureView},
};
use bevy_ecs::entity::Entity;
use std::borrow::Cow;
use thiserror::Error;

use super::{SlotValues, SlotInfo};

/// A command that signals the graph runner to run the sub graph corresponding to the `name`
/// with the specified `inputs` next.
pub struct RunSubGraph {
    pub name: Cow<'static, str>,
    pub inputs: SlotValues,
}

#[derive(Default)]
pub struct RunSubGraphs {
    commands: Vec<RunSubGraph>
}


impl RunSubGraphs {
    pub fn drain(self) -> impl Iterator<Item = RunSubGraph> {
        self.commands.into_iter()
    }

    pub fn run(&mut self, name: impl Into<Cow<'static, str>>, inputs: impl Into<SlotValues>) {
        self.commands.push(RunSubGraph {
            name: name.into(),
            inputs: inputs.into()
        });
    }
}


/// The context with all graph information required to run a [`Node`](super::Node).
/// This context is created for each node by the `RenderGraphRunner`.
///
/// The slot input can be read from here
pub struct RenderGraphContext<'a> {
    inputs: &'a SlotValues
}

impl<'a> RenderGraphContext<'a> {
    /// Creates a new render graph context.
    pub fn new(
        inputs: &'a SlotValues,
    ) -> Self {
        Self {
            inputs
        }
    }

    /// Returns the input slot values for the node.
    #[inline]
    pub fn inputs(&self) -> &SlotValues {
        &self.inputs
    }


    pub fn get_entity(&self, label: impl Into<SlotLabel>) -> Result<&Entity, SlotError> {
        
        let label = label.into();

        match self.inputs.get_value(&label)? {
            SlotValue::Entity(e) => Ok(e),
            val => {
                Err(SlotError::MismatchedSlotType { label: label.clone(), expected: SlotType::Entity, actual: val.slot_type() } )
            }
        }
    }


}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RunSubGraphError {
    #[error("tried to run a non-existent sub-graph")]
    MissingSubGraph(Cow<'static, str>),
    #[error("passed in inputs, but this sub-graph doesn't have any")]
    SubGraphHasNoInputs(Cow<'static, str>),
    #[error("sub graph (name: '{graph_name:?}') could not be run because slot '{slot_name}' at index {slot_index} has no value")]
    MissingInput {
        slot_index: usize,
        slot_name: Cow<'static, str>,
        graph_name: Cow<'static, str>,
    },
    #[error("attempted to use the wrong type for input slot")]
    MismatchedInputSlotType {
        graph_name: Cow<'static, str>,
        slot_index: usize,
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
}


#[derive(Error, Debug, Eq, PartialEq)]
pub enum SlotError {
    #[error("slot does not exist")]
    InvalidSlot(SlotLabel),
    #[error("attempted to retrieve the wrong type from input slot")]
    MismatchedSlotType {
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
}
