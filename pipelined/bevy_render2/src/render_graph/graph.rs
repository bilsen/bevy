use crate::{
    render_graph::{
        Edge, GraphContext, NodeId, NodeLabel, NodeRunError, NodeState, RenderGraphError,
    },
    renderer::RenderContext,
};
use bevy_ecs::{prelude::World, system::IntoSystem};
use bevy_reflect::{List, Uuid};
use bevy_utils::HashMap;
use std::{borrow::Cow, fmt::Debug};

use super::{BoxedNode, NodeInput, NodeResult};

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

pub struct MainRenderGraphId(RenderGraphId);
impl MainRenderGraphId {
    pub fn new(id: RenderGraphId) -> Self {
        Self(id)
    }
    pub fn id(&self) -> RenderGraphId {
        self.0
    }
}

impl Default for MainRenderGraphId {
    fn default() -> Self {
        Self(RenderGraphId::new())
    }
}

#[derive(Default)]
pub struct RenderGraphs {
    graphs: HashMap<RenderGraphId, RenderGraph>,
    names: HashMap<&'static str, RenderGraphId>
}

pub trait IntoGraphId {
    fn into_id(&self, graphs: &RenderGraphs) -> RenderGraphId;
}

impl<'a> IntoGraphId for &'a RenderGraphId {
    fn into_id(&self, graphs: &RenderGraphs) -> RenderGraphId {
        *self.clone()
    }
}

impl IntoGraphId for &'static str {
    fn into_id(&self, graphs: &RenderGraphs) -> RenderGraphId {
        *graphs.names.get(self).expect("No such name in render graph")
    }
}

impl RenderGraphs {
    pub fn get(&self, id: impl IntoGraphId) -> Option<&RenderGraph> {
        self.graphs.get(&id.into_id(self))
    }
    pub fn get_mut(&mut self, id: impl IntoGraphId) -> Option<&mut RenderGraph> {
        self.graphs.get_mut(&id.into_id(self))
    }

    pub fn insert(&mut self, name: impl Into<&'static str>, graph: RenderGraph) {
        
        let id=graph.id;
        self.graphs.insert(id, graph);
        self.names.insert(name.into(), id);
    }

    pub fn iter_graphs(&self) -> impl Iterator<Item=&RenderGraph> {
        self.graphs.iter().map(|(_key, graph)| graph)
    }

    pub fn iter_graphs_mut(&mut self) -> impl Iterator<Item=&mut RenderGraph> {
        self.graphs.iter_mut().map(|(_key, graph)| graph)
    }
}

pub struct RenderGraph {
    id: RenderGraphId,
    nodes: HashMap<NodeId, NodeState>,
    node_names: HashMap<Cow<'static, str>, NodeId>,
}

impl Default for RenderGraph {
    fn default() -> Self {
        RenderGraph {
            id: RenderGraphId::new(),
            nodes: HashMap::default(),
            node_names: HashMap::default(),
        }
    }
}

impl RenderGraph {
    pub fn update(&mut self, world: &mut World) {
        for node in self.nodes.values_mut() {
            node.system.apply_buffers(world)
        }
    }
    pub fn add_node<T>(&mut self, name: impl Into<Cow<'static, str>>, node: impl IntoSystem<NodeInput, NodeResult, T>) -> NodeId {
        println!("adding nodes");
        
        let id = NodeId::new();
        let name = name.into();
        let mut node_state = NodeState::new(id, Box::new(node.system()));
        node_state.name = Some(name.clone());
        self.nodes.insert(id, node_state);
        self.node_names.insert(name, id);
        id
    }

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
    pub fn add_edge(
        &mut self,
        output_node: impl Into<NodeLabel>,
        input_node: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let output_node_id = self.get_node_id(output_node)?;
        let input_node_id = self.get_node_id(input_node)?;

        let edge = Edge::new(input_node_id, output_node_id);

        {
            let output_node = self.get_node_state_mut(output_node_id)?;
            output_node.edges.add_output_edge(edge.clone())?;
        }
        let input_node = self.get_node_state_mut(input_node_id)?;
        input_node.edges.add_input_edge(edge)?;

        Ok(())
    }

    pub fn has_edge(&self, edge: &Edge) -> bool {
        let output_node_state = self.get_node_state(edge.get_output_node());
        let input_node_state = self.get_node_state(edge.get_input_node());
        if let Ok(output_node_state) = output_node_state {
            if output_node_state.edges.output_edges.contains(edge) {
                if let Ok(input_node_state) = input_node_state {
                    if input_node_state.edges.input_edges.contains(edge) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeState> {
        self.nodes.values()
    }

    pub fn iter_nodes_mut(&mut self) -> impl Iterator<Item = &mut NodeState> {
        self.nodes.values_mut()
    }

    pub fn iter_node_inputs(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .input_edges
            .iter()
            .map(|edge| (edge, edge.get_output_node()))
            .map(move |(edge, output_node_id)| {
                (edge, self.get_node_state(output_node_id).unwrap())
            }))
    }

    pub fn iter_node_outputs(
        &self,
        label: impl Into<NodeLabel>,
    ) -> Result<impl Iterator<Item = (&Edge, &NodeState)>, RenderGraphError> {
        let node = self.get_node_state(label)?;
        Ok(node
            .edges
            .output_edges
            .iter()
            .map(|edge| (edge, edge.get_input_node()))
            .map(move |(edge, input_node_id)| (edge, self.get_node_state(input_node_id).unwrap())))
    }

    pub fn id(&self) -> &RenderGraphId {
        &self.id
    }
}

impl Debug for RenderGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.iter_nodes() {
            writeln!(f, "{:?}", node.id)?;
            writeln!(f, "  in: {:?}", node.edges.input_edges)?;
            writeln!(f, "  out: {:?}", node.edges.output_edges)?;
        }

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::{
//         render_graph::{
//             Edge, Node, NodeId, NodeRunError, RenderGraph, RenderGraphContext, RenderGraphError,
//             SlotInfo, SlotType,
//         },
//         renderer::RenderContext,
//     };
//     use bevy_ecs::world::World;
//     use bevy_utils::HashSet;
//     use std::iter::FromIterator;

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
//         fn input(&self) -> Vec<SlotInfo> {
//             self.inputs.clone()
//         }

//         fn output(&self) -> Vec<SlotInfo> {
//             self.outputs.clone()
//         }

//         fn run(
//             &self,
//             _: &mut RenderGraphContext,
//             _: &mut RenderContext,
//             _: &World,
//         ) -> Result<(), NodeRunError> {
//             Ok(())
//         }
//     }

//     #[test]
//     fn test_graph_edges() {
//         let mut graph = RenderGraph::default();
//         let a_id = graph.add_node("A", TestNode::new(0, 1));
//         let b_id = graph.add_node("B", TestNode::new(0, 1));
//         let c_id = graph.add_node("C", TestNode::new(1, 1));
//         let d_id = graph.add_node("D", TestNode::new(1, 0));

//         graph.add_slot_edge("A", "out_0", "C", "in_0").unwrap();
//         graph.add_node_edge("B", "C").unwrap();
//         graph.add_slot_edge("C", 0, "D", 0).unwrap();

//         fn input_nodes(name: &'static str, graph: &RenderGraph) -> HashSet<NodeId> {
//             graph
//                 .iter_node_inputs(name)
//                 .unwrap()
//                 .map(|(_edge, node)| node.id)
//                 .collect::<HashSet<NodeId>>()
//         }

//         fn output_nodes(name: &'static str, graph: &RenderGraph) -> HashSet<NodeId> {
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
