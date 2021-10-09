use bevy_ecs::{
    archetype::Archetype,
    system::{In, IntoSystem, System},
    world::World,
};
use bevy_utils::{HashSet, Uuid};
use std::{borrow::Cow, fmt::Debug};
use thiserror::Error;
use wgpu::CommandEncoder;

use super::{GraphContext, RunSubGraphs, SlotInfos, SlotType};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NodeId(Uuid);

impl NodeId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        NodeId(Uuid::new_v4())
    }

    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

pub type RecordingNodeInput = (CommandEncoder, GraphContext);
pub type RecordingNodeOutput = Result<CommandEncoder, RecordingError>;
// A system that records to the command buffer
pub type RecordingNodeSystem = Box<dyn System<In = RecordingNodeInput, Out = RecordingNodeOutput>>;

pub type SubGraphRunNodeInput = GraphContext;
pub type SubGraphRunNodeOutput = Result<RunSubGraphs, SubGraphRunError>;
// A system that runs sub-graphs
pub type SubGraphRunNodeSystem =
    Box<dyn System<In = SubGraphRunNodeInput, Out = SubGraphRunNodeOutput>>;

#[derive(Error, Debug, Eq, PartialEq)]
pub enum NodeRunError {
    // #[error("encountered an input slot error")]
// InputSlotError(#[from] InputSlotError),
// #[error("encountered an output slot error")]
// OutputSlotError(#[from] OutputSlotError),
// #[error("encountered an error when running a sub-graph")]
// RunSubGraphError(#[from] RunSubGraphError),
}

#[derive(Debug, Default)]
pub struct Edges {
    pub dependencies: HashSet<NodeId>,
    pub dependants: HashSet<NodeId>,
}

impl Edges {
    pub(crate) fn add_dependency(&mut self, node: NodeId) {
        self.dependencies.insert(node);
    }

    pub(crate) fn add_dependant(&mut self, node: NodeId) {
        self.dependants.insert(node);
    }

    pub fn has_dependency(&self, edge: &NodeId) -> bool {
        self.dependencies.contains(edge)
    }

    pub fn has_dependant(&self, edge: &NodeId) -> bool {
        self.dependants.contains(edge)
    }

    pub fn iter_dependencies(&self) -> impl Iterator<Item = &NodeId> {
        self.dependencies.iter()
    }

    pub fn iter_dependants(&self) -> impl Iterator<Item = &NodeId> {
        self.dependants.iter()
    }
}

pub enum NodeSystem {
    RunSubGraphSystem(SubGraphRunNodeSystem),
    RecordingSystem(RecordingNodeSystem),
}

impl NodeSystem {
    pub fn new_archetype(&mut self, archetype: &Archetype) {
        match self {
            NodeSystem::RecordingSystem(ref mut system) => {
                system.new_archetype(archetype);
            }
            NodeSystem::RunSubGraphSystem(ref mut system) => {
                system.new_archetype(archetype);
            }
        }
    }

    pub fn initialize(&mut self, world: &mut World) {
        match self {
            NodeSystem::RecordingSystem(ref mut system) => {
                system.initialize(world);
            }
            NodeSystem::RunSubGraphSystem(ref mut system) => {
                system.initialize(world);
            }
        }
    }

    pub fn apply_buffers(&mut self, world: &mut World) {
        match self {
            NodeSystem::RecordingSystem(ref mut system) => {
                system.apply_buffers(world);
            }
            NodeSystem::RunSubGraphSystem(ref mut system) => {
                system.apply_buffers(world);
            }
        }
    }

    pub fn name(&self) -> Cow<'static, str> {
        match self {
            NodeSystem::RecordingSystem(ref system) => system.name(),
            NodeSystem::RunSubGraphSystem(ref system) => system.name(),
        }
    }
}

impl From<SubGraphRunNodeSystem> for NodeSystem {
    fn from(sys: SubGraphRunNodeSystem) -> Self {
        NodeSystem::RunSubGraphSystem(sys)
    }
}
impl From<RecordingNodeSystem> for NodeSystem {
    fn from(sys: RecordingNodeSystem) -> Self {
        NodeSystem::RecordingSystem(sys)
    }
}

pub struct RenderNode {
    pub id: NodeId,
    pub name: Cow<'static, str>,
    pub system: NodeSystem,
    pub system_name: Cow<'static, str>,
    pub edges: Edges,
    /// What slots need to be present in the render graph
    pub slot_requirements: SlotInfos,
}

impl Debug for RenderNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?} ({:?})", self.id, self.name)
    }
}

impl RenderNode {
    pub fn new(id: NodeId, system: NodeSystem, name: Cow<'static, str>) -> Self {
        RenderNode {
            id,
            name,
            system_name: system.name(),
            system,
            edges: Edges::default(),
            slot_requirements: SlotInfos::default(),
        }
    }

    pub fn recording_system_mut(&mut self) -> Option<&mut RecordingNodeSystem> {
        if let NodeSystem::RecordingSystem(ref mut system) = self.system {
            Some(system)
        } else {
            None
        }
    }
    pub fn sub_graph_run_system_mut(&mut self) -> Option<&mut SubGraphRunNodeSystem> {
        if let NodeSystem::RunSubGraphSystem(ref mut system) = self.system {
            Some(system)
        } else {
            None
        }
    }

    pub fn is_recording(&mut self) -> bool {
        matches!(self.system, NodeSystem::RecordingSystem(_))
    }

    pub fn is_sub_graph_run(&mut self) -> bool {
        !self.is_recording()
    }

    pub fn system_mut(&mut self) -> &mut NodeSystem {
        &mut self.system
    }

    pub fn id(&self) -> &NodeId {
        &self.id
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

pub struct RenderNodeBuilder {
    pub id: NodeId,
    pub name: Option<Cow<'static, str>>,
    pub system: NodeSystem,
    pub system_name: Cow<'static, str>,
    pub edges: Edges,
    pub slot_requirements: SlotInfos,
}

impl Default for RenderNodeBuilder {
    fn default() -> Self {
        let empty_system: NodeSystem = (Box::new(empty_node_system.system())
            as Box<dyn System<In = RecordingNodeInput, Out = RecordingNodeOutput>>)
            .into();
        let system_name = empty_system.name();
        Self {
            id: NodeId::new(),
            name: None,
            system_name,
            system: empty_system,
            edges: Edges::default(),
            slot_requirements: SlotInfos::default(),
        }
    }
}

impl RenderNodeBuilder {
    pub fn with_system<S, In, Out, Param>(mut self, sys: S) -> Self
    where
        S: IntoSystem<In, Out, Param>,
        Box<dyn System<In = In, Out = Out>>: Into<NodeSystem>,
    {
        let system = (Box::new(sys.system()) as Box<dyn System<In = In, Out = Out>>).into();
        self.system_name = system.name();
        self.system = system;
        self
    }

    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_requirement(
        mut self,
        name: impl Into<Cow<'static, str>>,
        slot_type: SlotType,
    ) -> Self {
        self.slot_requirements.add_slot(name, slot_type);
        self
    }

    pub fn build(self) -> RenderNode {
        RenderNode {
            id: self.id,
            name: self.name.unwrap_or_else(|| "<unnamed>".into()),
            system_name: self.system_name.clone(),
            system: self.system,
            edges: self.edges,
            slot_requirements: self.slot_requirements,
        }
    }
}

pub fn empty_node_system(
    In((render_context, _graph_context)): In<RecordingNodeInput>,
) -> RecordingNodeOutput {
    Ok(render_context)
}

#[derive(Debug)]
pub enum RecordingError {
    Error,
}

#[derive(Debug)]
pub enum SubGraphRunError {
    Error,
}
