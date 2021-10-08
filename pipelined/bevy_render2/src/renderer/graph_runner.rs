use bevy_ecs::{archetype::ArchetypeGeneration, world::World};
use bevy_reflect::List;
use bevy_utils::{
    tracing::{debug, info_span},
    HashMap, HashSet,
};
use smallvec::{smallvec, SmallVec};
use std::{borrow::Cow, collections::VecDeque};
use thiserror::Error;
use wgpu::CommandEncoder;

use crate::{
    render_graph::{
        Edge, GraphContext, NodeId, NodeRunError, NodeState, RenderGraph, RenderGraphId,
        RenderGraphs, RunSubGraph, SlotLabel, SlotType, SlotValue,
    },
    renderer::{RenderContext, RenderDevice},
};

use super::RenderQueue;

#[derive(Error, Debug)]
pub enum RenderGraphRunnerError {
    #[error(transparent)]
    NodeRunError(#[from] NodeRunError),
    #[error("node output slot not set (index {slot_index}, name {slot_name})")]
    EmptyNodeOutputSlot {
        type_name: &'static str,
        slot_index: usize,
        slot_name: Cow<'static, str>,
    },
    #[error("graph (name: '{graph_name:?}') could not be run because slot '{slot_name}' at index {slot_index} has no value")]
    MissingInput {
        slot_index: usize,
        slot_name: Cow<'static, str>,
        graph_name: Option<Cow<'static, str>>,
    },
    #[error("attempted to use the wrong type for input slot")]
    MismatchedInputSlotType {
        slot_index: usize,
        label: SlotLabel,
        expected: SlotType,
        actual: SlotType,
    },
}

pub(crate) struct RenderGraphRunner {
    archetype_generation: ArchetypeGeneration,
    initialized_nodes: HashSet<NodeId>,
}

impl Default for RenderGraphRunner {
    fn default() -> Self {
        Self {
            archetype_generation: ArchetypeGeneration::initial(),
            initialized_nodes: HashSet::default(),
        }
    }
}

impl RenderGraphRunner {
    fn update_archetypes(&mut self, world: &mut World, graphs: &mut RenderGraphs) {
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype in archetypes.archetypes()[archetype_index_range].iter() {
            let node_iterator = graphs
                .iter_graphs_mut()
                .flat_map(|graph| graph.iter_nodes_mut());
            for node in node_iterator {
                let system = node.system_mut();
                system.new_archetype(archetype);
            }
        }
    }

    fn initialize_nodes(&mut self, world: &mut World, graphs: &mut RenderGraphs) {
        let node_iterator = graphs
            .iter_graphs_mut()
            .flat_map(|graph| graph.iter_nodes_mut());
        for node in node_iterator {
            if !self.initialized_nodes.contains(&node.id) {
                node.system_mut().initialize(world);
                self.initialized_nodes.insert(node.id);
            }
        }
    }

    pub fn run_and_submit(
        &mut self,
        world: &mut World,
        // Resource RenderGraphs is temporarily removed from world. this is to disallow any funky cross-system mutation from occuring.
        render_graphs: &mut RenderGraphs,
        graph_id: RenderGraphId,
    ) -> Result<(), RenderGraphRunnerError> {
        self.initialize_nodes(world, render_graphs);
        self.update_archetypes(world, render_graphs);

        let render_device = world.get_resource::<RenderDevice>().unwrap().clone();
        let queue = world.get_resource::<RenderQueue>().unwrap().clone();

        let mut command_encoder =
            render_device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let span = info_span!("run_graph");
            let _guard = span.enter();
            command_encoder = self.run_graph(
                world,
                render_graphs,
                &graph_id,
                Some("Main graph".into()),
                command_encoder,
                GraphContext::default(),
            );
        }
        {
            let span = info_span!("submit_graph_commands");
            let _guard = span.enter();
            queue.submit(vec![command_encoder.finish()]);
        }
        Ok(())
    }

    fn run_graph(
        &mut self,
        world: &mut World,
        render_graphs: &mut RenderGraphs,
        graph_id: &RenderGraphId,
        graph_name: Option<Cow<'static, str>>,
        mut command_encoder: CommandEncoder,
        graph_context: GraphContext,
    ) -> CommandEncoder {
        debug!("-----------------");
        debug!("Begin Graph Run: {:?}", graph_name);
        debug!("-----------------");

        // Queue up nodes without inputs, which can be run immediately
        let mut node_queue: VecDeque<NodeId> = get_graph_mut(render_graphs, graph_id)
            .iter_nodes()
            .filter(|node| node.edges.dependencies.is_empty())
            .map(|state| state.id)
            .collect();

        let mut finished_nodes: HashSet<NodeId> = HashSet::default();

        'handle_node: while let Some(node_state_id) = node_queue.pop_front() {
            // skip nodes that are already processed
            if finished_nodes.contains(&node_state_id) {
                continue;
            }

            // Check if all dependencies have finished running
            for dependency_node in get_graph_mut(render_graphs, graph_id)
                .get_node_state(node_state_id)
                .unwrap()
                .edges
                .dependencies
                .iter()
            {
                if !finished_nodes.contains(&dependency_node) {
                    node_queue.push_back(node_state_id);
                    continue 'handle_node;
                }
            }
            let mut node_state = get_graph_mut(render_graphs, graph_id)
                .get_node_state_mut(node_state_id)
                .unwrap();

            // Run node TODO: Error handling
            let (output_encoder, sub_graph_runs) = node_state
                .system
                .run((command_encoder, graph_context.clone()), world)
                .unwrap();

            command_encoder = output_encoder;
            for run_sub_graph in sub_graph_runs.drain() {
                command_encoder = self.run_graph(
                    world,
                    render_graphs,
                    &run_sub_graph.id,
                    None,
                    command_encoder,
                    run_sub_graph.context,
                );
            }
            finished_nodes.insert(node_state_id);
            for output_node_id in
                get_graph_mut(render_graphs, graph_id).iter_dependants(&node_state_id)
            {
                node_queue.push_back(*output_node_id);
            }
        }

        debug!("finish graph: {:?}", graph_name);
        command_encoder
    }
}

fn get_graph_mut<'a>(
    render_graphs: &'a mut RenderGraphs,
    id: &RenderGraphId,
) -> &'a mut RenderGraph {
    render_graphs.get_mut(id).unwrap()
}
