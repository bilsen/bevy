use bevy_ecs::{archetype::ArchetypeGeneration, world::World};
use bevy_utils::{
    tracing::{debug, info_span},
    HashSet,
};
use std::{borrow::Cow, collections::VecDeque};
use thiserror::Error;
use wgpu::CommandEncoder;

use crate::{
    render_graph::{
        GraphContext, NodeId, NodeRunError, RenderGraphId, RenderGraphLabel, RenderGraphs, SlotType,
    },
    renderer::RenderDevice,
};

use super::RenderQueue;

#[derive(Error, Debug)]
pub enum RenderGraphRunnerError {
    #[error(transparent)]
    NodeRunError(#[from] NodeRunError),
    #[error("requested render graph not found")]
    MissingRenderGraph(RenderGraphLabel),
    #[error("node output slot not set (index {slot_index}, name {slot_name})")]
    EmptyNodeOutputSlot {
        type_name: &'static str,
        slot_index: usize,
        slot_name: Cow<'static, str>,
    },
    #[error(
        "graph (name: '{graph_name:?}') could not be run because slot '{slot_name}' has no value"
    )]
    MissingInput {
        slot_name: Cow<'static, str>,
        graph_name: Option<Cow<'static, str>>,
    },
    #[error("attempted to use the wrong type for input slot")]
    MismatchedInputSlotType {
        slot_name: Cow<'static, str>,
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
// TODO: One step for "expanding" graph, running systems which spawn sub graphs, and one step for running the resulting graph of "recording" nodes.
// Will simplyfy the execution model and allow for more paralellism between subgraphs.
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
        render_graphs: &mut RenderGraphs,
        main_graph_id: &RenderGraphId,
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
                main_graph_id,
                Some("Main graph".into()),
                command_encoder,
                GraphContext::default(),
            )?;
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
    ) -> Result<CommandEncoder, RenderGraphRunnerError> {
        debug!("-----------------");
        debug!("Begin Graph Run: {:?}", graph_name);
        debug!("-----------------");

        if render_graphs.get(*graph_id).is_none() {
            return Err(RenderGraphRunnerError::MissingRenderGraph(
                (*graph_id).into(),
            ));
        }

        // Queue up nodes without inputs, which can be run immediately
        let mut node_queue: VecDeque<NodeId> = render_graphs
            .get_mut(*graph_id)
            .unwrap()
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
            let node_state = render_graphs
                .get_mut(*graph_id)
                .unwrap()
                .get_node_mut(node_state_id)
                .unwrap();
            // Check if all dependencies have finished running
            for dependency_node in node_state.edges.iter_dependencies() {
                if !finished_nodes.contains(dependency_node) {
                    node_queue.push_back(node_state_id);
                    continue 'handle_node;
                }
            }

            // Run node TODO: Error handling

            let mut run_sub_graphs = None;
            if let Some(system) = node_state.recording_system_mut() {
                command_encoder = system
                    .run((command_encoder, graph_context.clone()), world)
                    .unwrap();
            } else if let Some(system) = node_state.sub_graph_run_system_mut() {
                run_sub_graphs = Some(system.run(graph_context.clone(), world).unwrap());
            }
            if let Some(sub_graph_runs) = run_sub_graphs {
                for run_sub_graph in sub_graph_runs.drain() {
                    command_encoder = self.run_graph(
                        world,
                        render_graphs,
                        &run_sub_graph.id,
                        None,
                        command_encoder,
                        run_sub_graph.context,
                    )?;
                }
            }
            finished_nodes.insert(node_state_id);
            for output_node_id in render_graphs
                .get_mut(*graph_id)
                .unwrap()
                .iter_dependants(&node_state_id)
            {
                node_queue.push_back(*output_node_id);
            }
        }

        debug!("finish graph: {:?}", graph_name);
        Ok(command_encoder)
    }
}
