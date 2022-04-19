use crate::render_graph::{Node, NodeId, NodeLabel, NodeState, RenderGraphError};
use bevy_ecs::prelude::World;
use bevy_reflect::Uuid;
use bevy_utils::HashMap;
use std::{borrow::Cow, fmt::Debug};

use super::{QueueGraphError, SlotInfos};

/// A render graph configures the modular, parallel and re-usable render logic.
/// It is a retained and stateless (nodes itself may have their internal state) structure,
/// which can not be modified while it is executed by the graph runner.
///
/// The `RenderGraphRunner` is responsible for executing the entire graph each frame.
///
/// It consists of two main components: [`Nodes`](NodeState)
///
/// Nodes are responsible for generating draw calls (`record`) and queuing other graphs for execution (`queue_graphs`).
/// Edges specify the order of recording.
///
/// ## Example
/// Here is a simple render graph example with two nodes connected by an edge.
/// ```
/// # use bevy_app::prelude::*;
/// # use bevy_ecs::prelude::World;
/// # use bevy_render::render_graph::{RenderGraph, Node, RenderGraphContext, NodeRunError};
/// # use bevy_render::renderer::RenderContext;
/// #
/// # struct MyNode;
/// #
/// # impl Node for MyNode {
/// #     fn record(&self, graph: &RenderGraphContext, render_context: &mut RenderContext, world: &World) -> Result<(), NodeRunError> {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// let mut graph = RenderGraph::new("my_graph");
/// graph.add_node("input_node", MyNode);
/// graph.add_node("output_node", MyNode);
/// graph.add_edge("output_node", "input_node").unwrap();
/// ```
pub struct RenderGraph {
    id: RenderGraphId,
    name: Cow<'static, str>,
    nodes: HashMap<NodeId, NodeState>,
    node_names: HashMap<Cow<'static, str>, NodeId>,
    slot_requirements: SlotInfos,
}

impl RenderGraph {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            id: RenderGraphId::new(),
            name: name.into(),
            nodes: Default::default(),
            node_names: Default::default(),
            slot_requirements: Default::default(),
        }
    }

    pub fn get_name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn id(&self) -> &RenderGraphId {
        &self.id
    }

    pub fn get_slot_requirements(&self) -> &SlotInfos {
        &self.slot_requirements
    }

    /// Updates all nodes and sub graphs of the render graph. Should be called before executing it.
    pub fn update(&mut self, world: &mut World) {
        for node in self.nodes.values_mut() {
            node.node.update(world);
            node.required_slots = node.node.slot_requirements();
        }

        self.update_requirements()
    }

    /// ads
    pub fn update_requirements(&mut self) {
        self.slot_requirements = SlotInfos::default();
        for node in self.nodes.values() {
            for requirement in node.required_slots.iter() {
                self.slot_requirements.insert(requirement.clone());
            }
        }
    }

    pub fn assert_matching_slots(&self, _infos: &SlotInfos) -> Result<(), QueueGraphError> {
        todo!()
    }

    /// Adds the `node` with the `name` to the graph.
    /// If the name is already present replaces it instead.
    pub fn add_node<T>(&mut self, name: impl Into<Cow<'static, str>>, node: T) -> NodeId
    where
        T: Node,
    {
        let id = NodeId::new();
        let name = name.into();
        let mut node_state = NodeState::new(id, node);
        node_state.name = name.clone();
        self.nodes.insert(id, node_state);
        self.node_names.insert(name, id);
        id
    }

    /// Removes the `node` with the `name` from the graph.
    /// If the name is does not exist, nothing happens.
    pub fn remove_node(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<(), RenderGraphError> {
        let name = name.into();
        if let Some(id) = self.node_names.remove(&name) {
            if let Some(node_state) = self.nodes.remove(&id) {
                // Remove all edges from other nodes to this one. Note that as we're removing this
                // node, we don't need to remove its input edges
                for input_edge in node_state.edges.input_edges().iter() {
                    match input_edge {
                        Edge::SlotEdge {
                            output_node,
                            output_index: _,
                            input_node: _,
                            input_index: _,
                        } => {
                            if let Ok(output_node) = self.get_node_state_mut(*output_node) {
                                output_node.edges.remove_output_edge(input_edge.clone())?;
                            }
                        }
                        Edge::NodeEdge {
                            input_node: _,
                            output_node,
                        } => {
                            if let Ok(output_node) = self.get_node_state_mut(*output_node) {
                                output_node.edges.remove_output_edge(input_edge.clone())?;
                            }
                        }
                    }
                }
                // Remove all edges from this node to other nodes. Note that as we're removing this
                // node, we don't need to remove its output edges
                for output_edge in node_state.edges.output_edges().iter() {
                    match output_edge {
                        Edge::SlotEdge {
                            output_node: _,
                            output_index: _,
                            input_node,
                            input_index: _,
                        } => {
                            if let Ok(input_node) = self.get_node_state_mut(*input_node) {
                                input_node.edges.remove_input_edge(output_edge.clone())?;
                            }
                        }
                        Edge::NodeEdge {
                            output_node: _,
                            input_node,
                        } => {
                            if let Ok(input_node) = self.get_node_state_mut(*input_node) {
                                input_node.edges.remove_input_edge(output_edge.clone())?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Retrieves the [`NodeState`] referenced by the `label`.
    pub fn get_node_state(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<&NodeState, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get(&node_id)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    /// Retrieves the [`NodeState`] referenced by the `label` mutably.
    pub fn get_node_state_mut(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut NodeState, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get_mut(&node_id)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    /// Retrieves the [`NodeId`] referenced by the `label`.
    pub fn get_node_id(&self, label: impl Into<NodeLabel>) -> Result<NodeId, RenderGraphError> {
        let label = label.into();
        match label {
            NodeLabel::Id(id) => Ok(id),
            NodeLabel::Name(ref name) => self
                .node_names
                .get(name)
                .cloned()
                .ok_or(RenderGraphError::InvalidNode(label)),
        }
    }

    /// Retrieves the [`Node`] referenced by the `label`.
    pub fn get_node<T>(&self, label: impl Into<NodeLabel>) -> Result<&T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state(label).and_then(|n| n.node())
    }

    /// Retrieves the [`Node`] referenced by the `label` mutably.
    pub fn get_node_mut<T>(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut T, RenderGraphError>
    where
        T: Node,
    {
        self.get_node_state_mut(label).and_then(|n| n.node_mut())
    }

    /// Adds the [`Edge`] to the graph. This guarantees that the `output_node`
    /// is run before the `input_node`.
    pub fn add_edge(
        &mut self,
        before_node: impl Into<NodeLabel>,
        after_node: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let before_node_id = self.get_node_id(before_node)?;
        let after_node_id = self.get_node_id(after_node)?;

        self.validate_edge(&before_node_id, &after_node_id)?;

        {
            let after_node = self.get_node_state_mut(after_node_id)?;
            after_node.add_dependency(before_node_id);
        }
        let before_node = self.get_node_state_mut(after_node_id)?;
        before_node.add_dependant(after_node_id);

        Ok(())
    }

    /// Removes the [`Edge::NodeEdge`] from the graph. If either node does not exist then nothing
    /// happens.
    pub fn remove_node_edge(
        &mut self,
        output_node: impl Into<NodeLabel>,
        input_node: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_node_id = self.get_node_id(output_node)?;
        let input_node_id = self.get_node_id(input_node)?;

        let edge = Edge::NodeEdge {
            output_node: output_node_id,
            input_node: input_node_id,
        };

        self.validate_edge(&edge, EdgeExistence::Exists)?;

        {
            let output_node = self.get_node_state_mut(output_node_id)?;
            output_node.edges.remove_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node_id)?;
        input_node.edges.remove_input_edge(edge)?;

        Ok(())
    }

    /// Verifies that the edge existence is as expected and
    /// checks that slot edges are connected correctly.
    pub fn validate_edge(
        &mut self,
        before: &NodeId,
        after: &NodeId,
    ) -> Result<(), RenderGraphError> {
        if self.has_edge(before, after) {
            return Err(RenderGraphError::EdgeAlreadyExists(*before, *after));
        }
        // TODO: Check for cycles

        Ok(())
    }

    /// Checks whether the `edge` already exists in the graph.
    pub fn has_edge(&self, before: &NodeId, after: &NodeId) -> bool {
        let after_node = match self.get_node_state(*after) {
            Err(_) => return false,
            Ok(v) => v,
        };
        let before_node = match self.get_node_state(*before) {
            Err(_) => return false,
            Ok(v) => v,
        };
        after_node.iter_dependencies().any(|id| id == before)
            || before_node.iter_dependants().any(|id| id == after)
    }

    /// Returns an iterator over the [`NodeStates`](NodeState).
    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeState> {
        self.nodes.values()
    }

    /// Returns an iterator over the [`NodeStates`](NodeState), that allows modifying each value.
    pub fn iter_nodes_mut(&mut self) -> impl Iterator<Item = &mut NodeState> {
        self.nodes.values_mut()
    }

    /// Returns an iterator over a tuple of the input edges and the corresponding output nodes
    /// for the node referenced by the label.
    pub fn iter_dependencies(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = &NodeState>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .iter_dependencies()
            .map(move |id| self.get_node_state(*id).unwrap()))
    }

    /// Returns an iterator over a tuple of the output edges and the corresponding input nodes
    /// for the node referenced by the label.
    pub fn iter_dependants(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = &NodeState>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .iter_dependants()
            .map(move |id| self.get_node_state(*id).unwrap()))
    }
}

impl Debug for RenderGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.iter_nodes() {
            writeln!(f, "{:?}", node.id)?;
            writeln!(f, "  requires: {:?}", node.required_slots)?;
        }

        Ok(())
    }
}

/// A [`RenderGraph`] identifier.
/// It automatically generates its own random uuid.
///
/// This id is used to reference the graph internally (queuing graphs, etc.).
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

/// The resource containing all the [`RenderGraph`]:s
#[derive(Default)]
pub struct RenderGraphs {
    graphs: HashMap<RenderGraphId, RenderGraph>,
    graph_names: HashMap<Cow<'static, str>, RenderGraphId>,
}

/// A [`GraphLabel`] is used to reference a [`NodeState`] by either its name or [`NodeId`]
/// inside the [`RenderGraph`](super::RenderGraph).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GraphLabel {
    Id(RenderGraphId),
    Name(Cow<'static, str>),
}

impl From<&GraphLabel> for GraphLabel {
    fn from(value: &GraphLabel) -> Self {
        value.clone()
    }
}

impl From<String> for GraphLabel {
    fn from(value: String) -> Self {
        GraphLabel::Name(value.into())
    }
}

impl From<&'static str> for GraphLabel {
    fn from(value: &'static str) -> Self {
        GraphLabel::Name(value.into())
    }
}

impl From<RenderGraphId> for GraphLabel {
    fn from(value: RenderGraphId) -> Self {
        GraphLabel::Id(value)
    }
}

impl RenderGraphs {
    /// Adds the `graph` with the `name` to the graph.
    /// If the name is already present replaces it instead.
    pub fn add_graph(&mut self, graph: RenderGraph) {
        self.graph_names.insert(graph.name.clone(), graph.id);
        self.graphs.insert(graph.id, graph);
    }

    pub fn get_graph_id(&self, label: GraphLabel) -> Option<RenderGraphId> {
        match label {
            GraphLabel::Id(i) => Some(i),
            GraphLabel::Name(n) => Some(*self.graph_names.get(&n)?),
        }
    }

    pub fn get_graph_name(&self, label: &RenderGraphId) -> Option<&Cow<'static, str>> {
        self.graphs.get(label).map(|g| &g.name)
    }

    /// Retrieves the sub graph corresponding to the `name`.
    pub fn get_graph(&self, label: impl Into<GraphLabel>) -> Option<&RenderGraph> {
        let id = self.get_graph_id(label.into())?;
        self.graphs.get(&id)
    }

    /// Retrieves the sub graph corresponding to the `name`.
    pub fn get_graph_mut(&mut self, label: impl Into<GraphLabel>) -> Option<&mut RenderGraph> {
        let id = self.get_graph_id(label.into())?;
        self.graphs.get_mut(&id)
    }

    pub fn update(&mut self, world: &mut World) {
        for graph in self.graphs.values_mut() {
            graph.update(world);
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use std::borrow::Cow;

//     use crate::{
//         render_graph::{
//             Edge, Node, NodeId, NodeRunError, RenderGraph, RenderGraphContext, RenderGraphError,
//             SlotInfo, SlotType, SlotInfos,
//         },
//         renderer::RenderContext,
//     };
//     use bevy_ecs::world::World;
//     use bevy_utils::HashSet;

//     #[derive(Debug)]
//     struct TestNode {
//         inputs: Vec<SlotInfo>,
//         outputs: Vec<SlotInfo>,
//     }

//     impl TestNode {
//         pub fn new(inputs: usize, outputs: usize) -> Self {
//             TestNode {
//                 inputs: (0..inputs)
//                     .map(|i| SlotInfo::new(format!("in_{}", i), SlotType::TextureView))
//                     .collect(),
//                 outputs: (0..outputs)
//                     .map(|i| SlotInfo::new(format!("out_{}", i), SlotType::TextureView))
//                     .collect(),
//             }
//         }
//     }

//     impl Node for TestNode {
//         fn slot_requirements(&self) -> SlotInfos {
//             self.inputs.clone().into()
//         }

//         fn record(
//             &self,
//             _: &RenderGraphContext,
//             _: &mut RenderContext,
//             _: &World,
//         ) -> Result<(), NodeRunError> {
//             Ok(())
//         }
//     }

//     #[test]
//     fn test_graph_edges() {
//         let mut graph = RenderGraph::new("");

//         let a_id = graph.add_node("A", TestNode::new(0, 1));
//         let b_id = graph.add_node("B", TestNode::new(0, 1));
//         let c_id = graph.add_node("C", TestNode::new(1, 1));
//         let d_id = graph.add_node("D", TestNode::new(1, 0));

//         graph.add_edge("B", "C").unwrap();
//         graph.add_edge("A", "C").unwrap();
//         graph.add_edge("C", "D").unwrap();

//         fn input_nodes(name: Cow<'static, str>, graph: &RenderGraph) -> HashSet<NodeId> {
//             graph
//                 .iter_node_inputs(name)
//                 .unwrap()
//                 .map(|(_edge, node)| node.id)
//                 .collect::<HashSet<NodeId>>()
//         }

//         fn output_nodes(name: Cow<'static, str>, graph: &RenderGraph) -> HashSet<NodeId> {
//             graph
//                 .iter_node_outputs(name)
//                 .unwrap()
//                 .map(|(_edge, node)| node.id)
//                 .collect::<HashSet<NodeId>>()
//         }

//         assert!(input_nodes("A", &graph).is_empty(), "A has no inputs");
//         assert!(
//             output_nodes("A", &graph) == HashSet::from_iter(vec![c_id]),
//             "A outputs to C"
//         );

//         assert!(input_nodes("B", &graph).is_empty(), "B has no inputs");
//         assert!(
//             output_nodes("B", &graph) == HashSet::from_iter(vec![c_id]),
//             "B outputs to C"
//         );

//         assert!(
//             input_nodes("C", &graph) == HashSet::from_iter(vec![a_id, b_id]),
//             "A and B input to C"
//         );
//         assert!(
//             output_nodes("C", &graph) == HashSet::from_iter(vec![d_id]),
//             "C outputs to D"
//         );

//         assert!(
//             input_nodes("D", &graph) == HashSet::from_iter(vec![c_id]),
//             "C inputs to D"
//         );
//         assert!(output_nodes("D", &graph).is_empty(), "D has no outputs");
//     }

//     #[test]
//     fn test_get_node_typed() {
//         struct MyNode {
//             value: usize,
//         }

//         impl Node for MyNode {
//             fn run(
//                 &self,
//                 _: &mut RenderGraphContext,
//                 _: &mut RenderContext,
//                 _: &World,
//             ) -> Result<(), NodeRunError> {
//                 Ok(())
//             }
//         }

//         let mut graph = RenderGraph::default();

//         graph.add_node("A", MyNode { value: 42 });

//         let node: &MyNode = graph.get_node("A").unwrap();
//         assert_eq!(node.value, 42, "node value matches");

//         let result: Result<&TestNode, RenderGraphError> = graph.get_node("A");
//         assert_eq!(
//             result.unwrap_err(),
//             RenderGraphError::WrongNodeType,
//             "expect a wrong node type error"
//         );
//     }

//     #[test]
//     fn test_slot_already_occupied() {
//         let mut graph = RenderGraph::default();

//         graph.add_node("A", TestNode::new(0, 1));
//         graph.add_node("B", TestNode::new(0, 1));
//         graph.add_node("C", TestNode::new(1, 1));

//         graph.add_slot_edge("A", 0, "C", 0).unwrap();
//         assert_eq!(
//             graph.add_slot_edge("B", 0, "C", 0),
//             Err(RenderGraphError::NodeInputSlotAlreadyOccupied {
//                 node: graph.get_node_id("C").unwrap(),
//                 input_slot: 0,
//                 occupied_by_node: graph.get_node_id("A").unwrap(),
//             }),
//             "Adding to a slot that is already occupied should return an error"
//         );
//     }

//     #[test]
//     fn test_edge_already_exists() {
//         let mut graph = RenderGraph::default();

//         graph.add_node("A", TestNode::new(0, 1));
//         graph.add_node("B", TestNode::new(1, 0));

//         graph.add_slot_edge("A", 0, "B", 0).unwrap();
//         assert_eq!(
//             graph.add_slot_edge("A", 0, "B", 0),
//             Err(RenderGraphError::EdgeAlreadyExists(Edge::SlotEdge {
//                 output_node: graph.get_node_id("A").unwrap(),
//                 output_index: 0,
//                 input_node: graph.get_node_id("B").unwrap(),
//                 input_index: 0,
//             })),
//             "Adding to a duplicate edge should return an error"
//         );
//     }
// }
