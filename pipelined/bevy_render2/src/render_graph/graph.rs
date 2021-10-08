use crate::render_graph::{Edge, NodeId, NodeLabel, RenderGraphError, RenderNode};
use bevy_ecs::prelude::World;
use bevy_reflect::Uuid;
use bevy_utils::HashMap;
use std::{borrow::Cow, fmt::Debug};

use super::SlotInfos;

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

pub struct MainRenderGraphId(pub RenderGraphId);

#[derive(Default)]
pub struct RenderGraphs {
    graphs: HashMap<RenderGraphId, RenderGraph>,
    names: HashMap<&'static str, RenderGraphId>,
}

pub trait IntoGraphId {
    fn into_id(&self, graphs: &RenderGraphs) -> RenderGraphId;
}

impl<'a> IntoGraphId for &'a RenderGraphId {
    fn into_id(&self, _graphs: &RenderGraphs) -> RenderGraphId {
        *self.clone()
    }
}

impl IntoGraphId for &'static str {
    fn into_id(&self, graphs: &RenderGraphs) -> RenderGraphId {
        *graphs
            .names
            .get(self)
            .expect("No such name in render graph")
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
        let id = graph.id;
        self.graphs.insert(id, graph);
        self.names.insert(name.into(), id);
    }

    pub fn iter_graphs(&self) -> impl Iterator<Item = &RenderGraph> {
        self.graphs.iter().map(|(_key, graph)| graph)
    }

    pub fn iter_graphs_mut(&mut self) -> impl Iterator<Item = &mut RenderGraph> {
        self.graphs.iter_mut().map(|(_key, graph)| graph)
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &RenderNode> {
        self.iter_graphs().flat_map(|graph| graph.iter_nodes())
    }
}

pub struct RenderGraph {
    id: RenderGraphId,
    name: Cow<'static, str>,
    nodes: HashMap<NodeId, RenderNode>,
    node_names: HashMap<Cow<'static, str>, NodeId>,
    /// What is required to run this graph
    slot_infos: SlotInfos,
}

impl Default for RenderGraph {
    fn default() -> Self {
        RenderGraph {
            name: "Graph".into(),
            id: RenderGraphId::new(),
            nodes: HashMap::default(),
            node_names: HashMap::default(),
            slot_infos: SlotInfos::default(),
        }
    }
}

impl RenderGraph {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        RenderGraph {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn update(&mut self, world: &mut World) {
        // Allow for nodes to have commands?
        // for node in self.nodes.values_mut() {
        //     node.system.apply_buffers(world)
        // }
    }
    pub fn add_node(&mut self, node: RenderNode) -> NodeId {
        let id = *node.id();
        let name = node.name().clone();

        for dependency in node.edges.iter_dependencies() {
            self.add_edge(*dependency, id).unwrap();
        }

        for dependant in node.edges.iter_dependants() {
            self.add_edge(id, *dependant).unwrap();
        }

        self.nodes.insert(id, node);
        self.node_names.insert(name, id);
        id
    }

    pub fn get_node(&self, label: impl Into<NodeLabel>) -> Result<&RenderNode, RenderGraphError> {
        let label = label.into();
        let node_id = self.get_node_id(&label)?;
        self.nodes
            .get(&node_id)
            .ok_or(RenderGraphError::InvalidNode(label))
    }

    pub fn get_node_mut(
        &mut self,
        label: impl Into<NodeLabel>,
    ) -> Result<&mut RenderNode, RenderGraphError> {
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
        dependency: impl Into<NodeLabel>,
        dependant: impl Into<NodeLabel>,
    ) -> Result<(), RenderGraphError> {
        let dependency_node_id = self.get_node_id(dependency)?;
        let dependant_node_id = self.get_node_id(dependant)?;

        {
            let dependency_node = self.get_node_mut(dependency_node_id)?;
            dependency_node.edges.add_dependant(dependant_node_id);
        }
        let dependant_node = self.get_node_mut(dependant_node_id)?;
        dependant_node.edges.add_dependency(dependency_node_id);

        Ok(())
    }

    pub fn iter_dependants(&self, id: &NodeId) -> impl Iterator<Item = &NodeId> {
        self.nodes.get(id).unwrap().edges.iter_dependants()
    }

    pub fn iter_dependencies(&self, id: &NodeId) -> impl Iterator<Item = &NodeId> {
        self.nodes.get(id).unwrap().edges.iter_dependencies()
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &RenderNode> {
        self.nodes.values()
    }

    pub fn iter_nodes_mut(&mut self) -> impl Iterator<Item = &mut RenderNode> {
        self.nodes.values_mut()
    }

    pub fn id(&self) -> &RenderGraphId {
        &self.id
    }

    pub fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }
}

impl Debug for RenderGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.iter_nodes() {
            writeln!(f, "{:?}", node.id)?;
            writeln!(f, "  dependencies: {:?}", node.edges.dependencies)?;
            writeln!(f, "  dependants: {:?}", node.edges.dependants)?;
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
