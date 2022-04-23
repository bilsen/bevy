use crate::render_graph::{
    BoxedNode, GraphRunError, NodeId, NodeState, QueueContext, RenderGraph, RenderGraphId,
    RenderGraphs, SlotValues,
};
use bevy_ecs::world::World;
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
#[cfg(feature = "trace")]
use std::ops::Deref;

use super::CachedData;

pub struct NodeLocation {
    pub graph_id: RenderGraphId,
    pub node_id: NodeId,
}

#[derive(Default)]
pub struct FlattenedGraph {
    /// Nodes topologically sorted with an index referring to their `SlotValues` instance
    pub nodes: Vec<(NodeLocation, usize)>,
    pub slot_values_set: Vec<SlotValues>,
}

impl FlattenedGraph {
    pub fn from(world: &World, graph: &RenderGraph, cached_data: &CachedData) -> FlattenedGraph {
        let mut builder = FlattenedGraphBuilder::new(world);
        builder
            .visit_graph(graph, SlotValues::default(), cached_data)
            .unwrap();
        builder.build()
    }
}

struct FlattenedGraphBuilder<'w> {
    world: &'w World,
    graphs: &'w RenderGraphs,
    graph_stack: Vec<RenderGraphId>,
    nodes: Vec<(NodeLocation, usize)>,
    slot_values_set: Vec<SlotValues>,
    position: Vec<usize>,
}

impl<'w> FlattenedGraphBuilder<'w> {
    fn new(world: &'w World) -> Self {
        FlattenedGraphBuilder {
            world,
            graphs: &*world.resource::<RenderGraphs>(),
            graph_stack: Default::default(),
            nodes: Default::default(),
            slot_values_set: Default::default(),
            position: Default::default(),
        }
    }
    fn build(self) -> FlattenedGraph {
        FlattenedGraph {
            nodes: self.nodes,
            slot_values_set: self.slot_values_set,
        }
    }

    fn current_slot_values(&self) -> &SlotValues {
        let current_position = self.position.last().unwrap();

        &self.slot_values_set[*current_position]
    }

    fn enter(&mut self, graph: &RenderGraph, slot_values: SlotValues) -> &mut Self {
        self.graph_stack.push(*graph.get_id());
        self.position.push(self.slot_values_set.len());
        self.slot_values_set.push(slot_values);
        self
    }

    fn visit_recording(&mut self, node: &NodeState) -> &mut Self {
        let graph_id = *self
            .graph_stack
            .last()
            .expect("Graph builder has entered graph");
        let node_id = *node.get_id();
        self.nodes.push((
            NodeLocation { graph_id, node_id },
            *self.position.last().unwrap(),
        ));
        self
    }

    fn visit_empty(&mut self, node: &NodeState) -> &mut Self {
        let graph_id = *self
            .graph_stack
            .last()
            .expect("Graph builder has entered graph");
        let node_id = *node.get_id();
        self.nodes.push((
            NodeLocation { graph_id, node_id },
            *self.position.last().unwrap(),
        ));
        self
    }

    fn visit_graph(
        &mut self,
        graph: &RenderGraph,
        slot_values: SlotValues,
        cached_data: &CachedData,
    ) -> Result<(), GraphRunError> {
        self.enter(graph, slot_values);

        let nodes = cached_data.sorted_nodes[graph.get_id()].iter();

        for node_id in nodes {
            let node = graph.get_node(node_id);

            match node.get_function() {
                BoxedNode::Queue(queue_function) => {
                    let current_slot_values = self.current_slot_values();

                    let mut queue_context = QueueContext::new(self.graphs);
                    // println!("Running node {:?}", node.get_label());
                    queue_function
                        .queue(current_slot_values, &mut queue_context, self.world)
                        .map_err(|node_error| GraphRunError::NodeRunError {
                            graph: format!("{:?}", graph.get_label()).into(),
                            node: format!("{:?}", node.get_label()).into(),
                            node_error,
                        })?;

                    for (label, new_slot_values) in queue_context.finish().drain(..) {
                        let new_graph = self.graphs.get(label.as_ref()).unwrap();
                        // println!("Visiting graph {:?}", label);
                        self.visit_graph(new_graph, new_slot_values, cached_data)?;
                    }
                    self.visit_empty(node);
                }
                BoxedNode::Record(_) => {
                    self.visit_recording(node);
                }
                BoxedNode::Empty(_) => {
                    self.visit_empty(node);
                }
            }
        }
        self.exit();
        Ok(())
    }

    fn exit(&mut self) -> &mut Self {
        self.graph_stack
            .pop()
            .expect("Graph builder has entered graph");
        self.position.pop();
        self
    }
}
