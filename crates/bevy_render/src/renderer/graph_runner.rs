use bevy_ecs::world::World;
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::{HashMap, HashSet};
#[cfg(feature = "trace")]
use std::ops::Deref;

use crate::render_graph::{
    BoxedNode, GraphRunError, NodeId, RenderGraph, RenderGraphId, RenderGraphLabel, RenderGraphs,
};

use super::{flatten_graph::FlattenedGraph, RenderContext, RenderDevice};

#[derive(Default)]
pub struct ParalellRenderGraphRunner {
    cached_data: CachedData,
}

#[derive(Default)]
pub struct CachedData {
    pub sorted_nodes: HashMap<RenderGraphId, Vec<NodeId>>,
}

impl ParalellRenderGraphRunner {
    pub fn run(
        &mut self,
        main_graph_label: &impl RenderGraphLabel,
        render_device: RenderDevice,
        queue: &wgpu::Queue,
        world: &World,
    ) -> Result<(), GraphRunError> {
        let command_encoder =
            render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut render_context = RenderContext {
            render_device,
            command_encoder,
        };

        let graphs = world.resource::<RenderGraphs>();
        // fn run(&mut self, world: &World, main_graph_label: impl RenderGraphLabel, graphs: &RenderGraphs) {
        self.cached_data = rebuild_cached_data(graphs);

        let main_graph = graphs.get(main_graph_label).expect("Main graph exists");
        let flattened = FlattenedGraph::from(world, main_graph, &self.cached_data);
        let FlattenedGraph {
            nodes,
            slot_values_set,
        } = flattened;

        for (location, slot_value_index) in nodes {
            let node = graphs
                .get_node(&location.graph_id, &location.node_id)
                .unwrap();
            match node.get_function() {
                BoxedNode::Empty(_) | BoxedNode::Queue(_) => {}
                BoxedNode::Record(recording_node) => {
                    recording_node
                        .record(
                            &slot_values_set[slot_value_index],
                            &mut render_context,
                            world,
                        )
                        .map_err(|node_error| {
                            let graph_label =
                                graphs.get_by_id(&location.graph_id).unwrap().get_label();
                            let node_label = node.get_label();
                            GraphRunError::NodeRunError {
                                graph: format!("{:?}", graph_label).into(),
                                node: format!("{:?}", node_label).into(),
                                node_error,
                            }
                        })?;
                }
            }
        }

        let command_buffer = render_context.command_encoder.finish();
        queue.submit(vec![command_buffer]);

        Ok(())
        // Split nodes into sets equaling the number of workgroups
    }
}

fn rebuild_cached_data(graphs: &RenderGraphs) -> CachedData {
    let mut cached_data = CachedData::default();
    for graph in graphs.iter() {
        cached_data
            .sorted_nodes
            .insert(*graph.get_id(), topologically_sort_graph(graph));
    }

    cached_data
}

fn topologically_sort_graph(graph: &RenderGraph) -> Vec<NodeId> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for node in graph.iter_nodes() {
        let id = node.get_id();
        visit(graph, id, &mut seen, &mut result);
    }

    result
}

fn visit(
    graph: &RenderGraph,
    id: &NodeId,
    seen: &mut HashSet<NodeId>,
    sorted_list: &mut Vec<NodeId>,
) {
    if seen.contains(id) {
        return;
    }
    // println!("Visiting {:?}", id);
    seen.insert(*id);

    for before in graph.before(id) {
        visit(graph, before, seen, sorted_list);
    }

    sorted_list.push(*id);
}
