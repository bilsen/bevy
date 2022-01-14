use crate::render_graph::{SlotType, SlotValue};
use bevy_ecs::entity::Entity;
use std::borrow::Cow;
use thiserror::Error;

use super::{GraphLabel, RenderGraphId, RenderGraphs, SlotValues};

/// A command that signals the graph runner to run the sub graph corresponding to the `name`
/// with the specified `inputs` next.
pub struct QueueGraph {
    pub id: RenderGraphId,
    pub inputs: SlotValues,
}

#[derive(Default)]
pub struct QueueGraphs {
    commands: Vec<QueueGraph>,
}

impl QueueGraphs {
    pub fn drain(self) -> impl Iterator<Item = QueueGraph> {
        self.commands.into_iter()
    }

    pub fn queue(
        &mut self,
        ctx: &RenderGraphContext,
        label: impl Into<GraphLabel>,
        into_inputs: impl Into<SlotValues>,
    ) -> Result<(), QueueGraphError> {
        // TODO: Assert that the inputs match the graph

        let label = label.into();

        let graph = ctx
            .graphs
            .get_graph(label.clone())
            .ok_or_else(|| QueueGraphError::MissingGraph(label))?;

        let inputs = into_inputs.into();

        let infos = inputs.get_infos();

        let requirements = graph.get_slot_requirements();

        for provided in infos.iter() {
            let needed = requirements
                .get(&provided.name)
                .ok_or(QueueGraphError::UnusedInput {
                    slot_name: provided.name.clone(),
                    graph_name: graph.get_name().clone(),
                })?;
            if needed.slot_type != provided.slot_type {
                return Err(QueueGraphError::MismatchedSlotType {
                    graph_name: graph.get_name().clone(),
                    label: needed.name.clone(),
                    expected: needed.slot_type.clone(),
                    actual: provided.slot_type.clone(),
                });
            }
        }

        self.commands.push(QueueGraph {
            id: *graph.id(),
            inputs,
        });

        Ok(())
    }
}

/// The context with all graph information required to run a [`Node`](super::Node).
/// This context is created for each node by the `RenderGraphRunner`.
///
/// The slot input can be read from here
#[derive(Clone)]
pub struct RenderGraphContext<'a> {
    inputs: SlotValues,
    graphs: &'a RenderGraphs,
}

impl<'a> RenderGraphContext<'a> {
    /// Creates a new render graph context.
    pub fn new(inputs: SlotValues, graphs: &'a RenderGraphs) -> Self {
        Self { inputs, graphs }
    }

    /// Returns the input slot values for the node.
    #[inline]
    pub fn inputs(&self) -> &SlotValues {
        &self.inputs
    }

    pub fn get_entity(&self, label: impl Into<&'static str>) -> Result<&Entity, SlotError> {
        let label = label.into();

        match self.inputs.get_value(&label)? {
            SlotValue::Entity(e) => Ok(e),
            val => Err(SlotError::MismatchedSlotType {
                label,
                expected: SlotType::Entity,
                actual: val.slot_type(),
            }),
        }
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum QueueGraphError {
    #[error("tried to run a non-existent graph")]
    MissingGraph(GraphLabel),
    #[error("graph (name: '{graph_name:?}') could not be queued because slot '{slot_name}' has no value")]
    MissingInput {
        slot_name: Cow<'static, str>,
        graph_name: Cow<'static, str>,
    },
    #[error("attempted to use the wrong type for a slot")]
    MismatchedSlotType {
        graph_name: Cow<'static, str>,
        label: Cow<'static, str>,
        expected: SlotType,
        actual: SlotType,
    },
    #[error("graph (name: '{graph_name:?}') could not be queued because input '{slot_name}' does not match any slot requirement")]
    UnusedInput {
        slot_name: Cow<'static, str>,
        graph_name: Cow<'static, str>,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum SlotError {
    #[error("slot does not exist")]
    InvalidSlot(&'static str),
    #[error("attempted to retrieve the wrong type from input slot")]
    MismatchedSlotType {
        label: &'static str,
        expected: SlotType,
        actual: SlotType,
    },
}
