use bevy_ecs::prelude::World;
use bevy_reflect::Uuid;
use bevy_utils::{define_label, HashMap, HashSet};

use super::{
    node::{BoxedRenderNodeLabel, NodeId, NodeState, QueueNode, RecordingNode, RenderNodeLabel},
    slot::SlotRequirements,
    Node, NodeAddError, RenderGraphError,
};

pub use bevy_render_macros::RenderGraphLabel;
define_label!(RenderGraphLabel);

pub(crate) type BoxedRenderGraphLabel = Box<dyn RenderGraphLabel>;

/// A [`RenderGraph`] identifier.
/// It automatically generates its own random uuid.
///
/// This id is used to reference the graph internally (edges, etc).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RenderGraphId(Uuid);

impl RenderGraphId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        RenderGraphId(Uuid::new_v4())
    }

    pub fn uuid(&self) -> &Uuid {
        &self.0
    }
}

/// The resource containing the render graphs. Graphs can be identified by labels or Id:s
#[derive(Default)]
pub struct RenderGraphs {
    graphs: HashMap<RenderGraphId, RenderGraph>,
    labels: HashMap<BoxedRenderGraphLabel, RenderGraphId>,
}

impl RenderGraphs {
    pub fn get_id(&self, label: &dyn RenderGraphLabel) -> Result<&RenderGraphId, RenderGraphError> {
        let boxed_label: BoxedRenderGraphLabel = label.dyn_clone();
        self.labels
            .get(label)
            .ok_or_else(|| RenderGraphError::LabelError(format!("{:#?}", boxed_label).into()))
    }

    pub fn get_mut_by_id(
        &mut self,
        id: &RenderGraphId,
    ) -> Result<&mut RenderGraph, RenderGraphError> {
        self.graphs
            .get_mut(&id)
            .ok_or_else(|| RenderGraphError::IdError(*id))
    }

    pub fn get_by_id(&self, id: &RenderGraphId) -> Result<&RenderGraph, RenderGraphError> {
        self.graphs
            .get(&id)
            .ok_or_else(|| RenderGraphError::IdError(*id))
    }

    pub fn get(&self, label: &dyn RenderGraphLabel) -> Result<&RenderGraph, RenderGraphError> {
        let id = *self.get_id(label)?;
        self.get_by_id(&id)
    }

    pub fn get_mut(
        &mut self,
        label: &impl RenderGraphLabel,
    ) -> Result<&mut RenderGraph, RenderGraphError> {
        let id = *self.get_id(label)?;
        self.get_mut_by_id(&id)
    }

    pub fn get_node(
        &self,
        graph_id: &RenderGraphId,
        node_id: &NodeId,
    ) -> Result<&NodeState, RenderGraphError> {
        Ok(self.get_by_id(graph_id)?.get_node(node_id))
    }
    pub fn insert(&mut self, graph: RenderGraph) {
        self.labels.insert(graph.label.clone(), graph.id);
        self.graphs.insert(graph.id, graph);
    }

    pub fn iter(&self) -> impl Iterator<Item = &RenderGraph> {
        self.graphs.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RenderGraph> {
        self.graphs.values_mut()
    }

    pub fn update(&mut self, world: &mut World) {
        for graph in self.iter_mut() {
            for node in graph.iter_nodes_mut() {
                node.get_function_mut().update(world);
            }
        }
    }
}

pub struct RenderGraph {
    id: RenderGraphId,
    label: BoxedRenderGraphLabel,
    nodes: HashMap<NodeId, NodeState>,
    before: HashMap<NodeId, HashSet<NodeId>>,
    after: HashMap<NodeId, HashSet<NodeId>>,
    labels: HashMap<BoxedRenderNodeLabel, NodeId>,
    sorted_nodes: Vec<NodeId>,
    requirements: SlotRequirements,
}

impl RenderGraph {
    pub fn new(label: impl RenderGraphLabel, requirements: SlotRequirements) -> Self {
        let label = Box::new(label);
        Self {
            nodes: Default::default(),
            labels: Default::default(),
            id: RenderGraphId::new(),
            before: Default::default(),
            after: Default::default(),
            sorted_nodes: Vec::new(),
            label,
            requirements,
        }
    }

    fn sort_nodes(&mut self) -> bool {
        let sorted_nodes = Vec::new();
        let mut in_degrees: HashMap<_, _> = self
            .nodes
            .iter()
            .map(|(id, _)| (*id, self.before(id).len()))
            .collect();
        let mut queue: Vec<_> = in_degrees
            .iter()
            .filter_map(|(id, len)| (*len == 0).then(|| *id))
            .collect();

        while let Some(id) = queue.pop() {
            self.sorted_nodes.push(id);
            for after_id in self.after[&id].iter() {
                in_degrees.get_mut(after_id).map(|not_seen| *not_seen -= 1);
                if in_degrees[after_id] == 0 {
                    queue.push(*after_id);
                }
            }
        }
        if sorted_nodes.len() != self.nodes.len() {
            return false;
        }
        self.sorted_nodes = sorted_nodes;
        true
    }
    pub fn requirements(&self) -> &SlotRequirements {
        &self.requirements
    }

    pub fn get_node_id(&self, label: impl RenderNodeLabel) -> Result<&NodeId, RenderGraphError> {
        let boxed_label: BoxedRenderNodeLabel = Box::new(label);
        self.labels
            .get(&boxed_label)
            .ok_or_else(|| RenderGraphError::NodeLabelError(format!("{:#?}", boxed_label).into()))
    }

    pub fn get_node(&self, id: &NodeId) -> &NodeState {
        self.nodes.get(id).expect("Node exists")
    }

    pub fn get_label(&self) -> &BoxedRenderGraphLabel {
        &self.label
    }
    pub fn get_id(&self) -> &RenderGraphId {
        &self.id
    }

    pub fn add_recording_node(
        &mut self,
        label: impl RenderNodeLabel,
        node: impl RecordingNode,
    ) -> Result<&mut Self, NodeAddError> {
        let new_node = NodeState::from_recording(node, label);
        self.add_node(new_node)
    }

    pub fn add_queueing_node(
        &mut self,
        label: impl RenderNodeLabel,
        node: impl QueueNode,
    ) -> Result<&mut Self, NodeAddError> {
        let new_node = NodeState::from_queue(node, label);
        self.add_node(new_node)
    }

    pub fn add_empty_node(
        &mut self,
        label: impl RenderNodeLabel,
    ) -> Result<&mut Self, NodeAddError> {
        self.add_node(NodeState::from_empty(EmptyNode, label))
    }

    fn add_node(&mut self, state: NodeState) -> Result<&mut Self, NodeAddError> {
        let id = *state.get_id();
        // TODO: Validate node requirements.
        if let Some(error) = state
            .requirements()
            .get_slot_requirement_error(self.requirements())
        {
            return Err(error.into());
        }
        let label = state.get_label().clone();

        self.nodes.insert(id, state);
        self.labels.insert(label, id);
        self.after.insert(id, HashSet::default());
        self.before.insert(id, HashSet::default());
        self.sort_nodes(); 
        Ok(self)
    }

    pub fn add_edge(
        &mut self,
        before: impl RenderNodeLabel,
        after: impl RenderNodeLabel,
    ) -> Result<&mut Self, RenderGraphError> {
        // TODO: check for loops
        let before_id = *self.get_node_id(before)?;
        let after_id = *self.get_node_id(after)?;

        self.after
            .entry(before_id)
            .or_insert(HashSet::default())
            .insert(after_id);
        self.before
            .entry(after_id)
            .or_insert(HashSet::default())
            .insert(before_id);

        if !self.sort_nodes() {
            Err(RenderGraphError::EdgeAddError {
                graph: format!("{:?}", self.label).into(),
                before: format!("{:?}", self.nodes[&before_id].get_label()).into(),
                after: format!("{:?}", self.nodes[&after_id].get_label().dyn_clone()).into(),
            })
        } else {
            Ok(self)
        }
    }
    /// Removes edge between nodes. If it already exists it will do nothing
    pub fn remove_edge(
        &mut self,
        before: impl RenderNodeLabel,
        after: impl RenderNodeLabel,
    ) -> Result<&mut Self, RenderGraphError> {
        let before_id = *self.get_node_id(before)?;
        let after_id = *self.get_node_id(after)?;

        self.after
            .entry(before_id)
            .or_insert(HashSet::default())
            .take(&after_id);
        self.before
            .entry(after_id)
            .or_insert(HashSet::default())
            .take(&before_id);
        // Removing an edge will not invalidate node sort
        Ok(self)
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeState> {
        self.nodes.values()
    }

    pub fn iter_nodes_mut(&mut self) -> impl Iterator<Item = &mut NodeState> {
        self.nodes.values_mut()
    }

    pub fn after(&self, id: &NodeId) -> &HashSet<NodeId> {
        &self.after[id]
    }

    pub fn before(&self, id: &NodeId) -> &HashSet<NodeId> {
        &self.before[id]
    }
}

struct EmptyNode;

impl Node for EmptyNode {}
