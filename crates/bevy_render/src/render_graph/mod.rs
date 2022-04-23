mod graph;
mod node;
mod slot;

pub use graph::*;
pub use node::*;
pub use slot::*;

/// The label for the graph that begins execution
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MainRenderGraph;

/// Can be done with derive macro outside this crate
impl RenderGraphLabel for MainRenderGraph {
    fn dyn_clone(&self) -> Box<dyn RenderGraphLabel> {
        Box::new(self.clone())
    }
}

use std::borrow::Cow;

use thiserror::Error;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RenderGraphError {
    #[error("An error occured on a slot in graph")]
    SlotError {
        graph: Cow<'static, str>,
        error: SlotError,
    },

    #[error("Graph with label doesn't exist")]
    LabelError(Cow<'static, str>),
    #[error("Graph with id doesn't exist")]
    IdError(RenderGraphId),

    #[error("Graph doesn't contain node with label")]
    NodeLabelError(Cow<'static, str>),

    #[error("Error adding node to graph {graph}")]
    NodeAddError {
        graph: Cow<'static, str>,
        #[source]
        node_error: NodeAddError,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum NodeAddError {
    #[error("Slot requirement error")]
    SlotRequirementError(#[from] SlotRequirementError),
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum SlotError {
    #[error("A node was added that had a conflicting slot type")]
    SlotConflict {
        slot_name: Cow<'static, str>,
        current_type: Cow<'static, str>,
        new_type: Cow<'static, str>,
    },

    #[error("A slot was filled with the wrong data type")]
    SlotTypeError {
        slot_name: Cow<'static, str>,
        expected_type: Cow<'static, str>,
        actual_type: Cow<'static, str>,
    },
    #[error("Slot `{0}` doesn't exist")]
    SlotNameError(Cow<'static, str>),
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum NodeError {
    #[error("Slot error")]
    SlotError(#[from] SlotError),
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum GraphRunError {
    #[error("Node run error in graph {graph} and node {node}")]
    NodeRunError {
        graph: Cow<'static, str>,
        node: Cow<'static, str>,
        #[source]
        node_error: NodeRunError,
    },
}
#[derive(Error, Debug, Eq, PartialEq)]
pub enum NodeRunError {
    #[error("Queueing error")]
    QueueingError(#[from] QueueingError),
    #[error("Recording error")]
    RecordingError(#[from] RecordingError),

    #[error("Slot error")]
    SlotError(#[from] SlotError),
}

// #[derive(Error, Debug, Eq, PartialEq)]
// pub enum GraphRunError {
//     QueueingError(#[from] QueueingError)
// }

#[derive(Error, Debug, Eq, PartialEq)]
pub enum QueueingError {
    #[error("Render graph {0} not found")]
    LabelError(Cow<'static, str>),
    #[error("Label value error for graph {render_graph}")]
    SlotValueError {
        render_graph: Cow<'static, str>,
        #[source]
        value_error: SlotValueError,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum SlotValueError {
    #[error("No slot with name {name} exists")]
    SlotDoesntExist { name: Cow<'static, str> },
    #[error(
        "Slot with name {name} was has wrong type. Actual type {actual} expected type {expected}"
    )]
    SlotTypeError {
        name: Cow<'static, str>,
        actual: Cow<'static, str>,
        expected: Cow<'static, str>,
    },
    #[error("Slot with name {name} was provided the wrong type. Actual type {actual} recieved type {provided}")]
    SlotProvidedTypeError {
        name: Cow<'static, str>,
        actual: Cow<'static, str>,
        provided: Cow<'static, str>,
    },
    #[error("Slot {name} was not provided a value")]
    SlotNotProvided { name: Cow<'static, str> },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum SlotRequirementError {
    #[error("Slot requirement {name} is not satisfied")]
    SlotRequirementDoesntExist { name: Cow<'static, str> },
    #[error("Slot requirement {name} is not satisfied with the correct type. Type required {provided}, type found {actual}")]
    SlotRequirementTypeError {
        name: Cow<'static, str>,
        actual: Cow<'static, str>,
        provided: Cow<'static, str>,
    },
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RecordingError {
    #[error("Label value error for graph {render_graph}")]
    SlotValueError {
        render_graph: Cow<'static, str>,
        #[source]
        value_error: SlotValueError,
    },
}
