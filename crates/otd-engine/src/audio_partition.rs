use std::collections::HashSet;

use otd_chop::ops::is_rt_audio_sink;
use otd_chop::rt::wiring::{rt_inputs, rt_sinks, runs_on_worker};
use otd_core::{Connector, Graph, NodeId};

#[derive(Clone, Debug, Default)]
pub struct AudioPartition {
    /// Terminals the worker drives, clock owner first.
    pub sinks: Vec<NodeId>,
    /// Sinks plus every operator the worker runs for them.
    pub audio_nodes: HashSet<NodeId>,
    /// Frame-cooked producers the worker receives through a bridge.
    pub bridge_nodes: HashSet<NodeId>,
    /// Worker operators something on the frame side still reads, so they
    /// keep frame-cooking too.
    pub frame_read_nodes: HashSet<NodeId>,
    pub generation: u64,
}

pub fn graph_audio_generation(graph: &Graph) -> u64 {
    graph.audio_topology_generation()
}

pub fn partition_audio_domain(graph: &Graph, generation: u64) -> AudioPartition {
    let sinks = rt_sinks(graph);
    let mut audio_nodes: HashSet<NodeId> = sinks.iter().copied().collect();
    let mut bridge_nodes = HashSet::new();
    let mut stack = sinks.clone();
    while let Some(id) = stack.pop() {
        for (_, up) in rt_inputs(graph, id) {
            if !runs_on_worker(graph, up) {
                bridge_nodes.insert(up);
            } else if audio_nodes.insert(up) {
                stack.push(up);
            }
        }
    }
    let frame_read_nodes = frame_readers(graph, &audio_nodes);
    AudioPartition {
        sinks,
        audio_nodes,
        bridge_nodes,
        frame_read_nodes,
        generation,
    }
}

fn frame_readers(graph: &Graph, audio_nodes: &HashSet<NodeId>) -> HashSet<NodeId> {
    let mut read = HashSet::new();
    for id in graph.walk() {
        let node = graph.node(id);
        if audio_nodes.contains(&id) || node.flags.bypass || node.connector != Connector::None {
            continue;
        }
        for (_, up) in rt_inputs(graph, id) {
            if audio_nodes.contains(&up) {
                read.insert(up);
            }
        }
    }
    for id in audio_nodes {
        if graph.node(*id).flags.viewer && !is_rt_audio_sink(&graph.node(*id).op_type) {
            read.insert(*id);
        }
    }
    read
}

pub fn node_in_audio_domain(part: &AudioPartition, id: NodeId) -> bool {
    part.audio_nodes.contains(&id)
}

/// While the worker is armed, it alone renders its operators; the frame
/// cook keeps only those a frame operator reads.
pub fn skip_frame_chop_cook(part: &AudioPartition, rt_armed: bool, id: NodeId) -> bool {
    rt_armed && part.audio_nodes.contains(&id) && !part.frame_read_nodes.contains(&id)
}
