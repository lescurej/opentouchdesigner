use std::collections::HashSet;

use otd_chop::io::{AUDIO_FILE, AUDIO_IN, AUDIO_VST, SPECTRUM};
use otd_chop::ops::{is_rt_audio_sink, is_rt_safe_chop, is_rt_worker_sink, primary_rt_audio_sink, NULL};
use otd_core::{Graph, NodeId};

#[derive(Clone, Debug, Default)]
pub struct AudioPartition {
    pub audio_nodes: HashSet<NodeId>,
    pub bridge_nodes: HashSet<NodeId>,
    pub tap_nodes: HashSet<NodeId>,
    /// `Audio Device Out` owned by the RT worker, if any.
    pub primary_sink: Option<NodeId>,
    /// Upstream of non-primary RT sinks — must keep frame-cooking when RT is armed.
    pub frame_cook_nodes: HashSet<NodeId>,
    pub generation: u64,
}

pub fn graph_audio_generation(graph: &Graph) -> u64 {
    graph.audio_topology_generation()
}

pub fn partition_audio_domain(graph: &Graph, generation: u64) -> AudioPartition {
    let mut sinks = Vec::new();
    for id in graph.walk() {
        if graph.node(id).flags.bypass {
            continue;
        }
        if is_rt_worker_sink(graph.node(id).op_type.as_str()) {
            sinks.push(id);
        }
    }
    let mut audio_nodes = HashSet::new();
    let mut bridge_nodes = HashSet::new();
    for sink in sinks {
        audio_nodes.insert(sink);
        walk_upstream_rt(graph, sink, &mut audio_nodes, &mut bridge_nodes);
    }
    for id in audio_nodes.clone() {
        for input in &graph.node(id).inputs {
            if let Some(up) = *input
                && !audio_nodes.contains(&up)
                && bridge_nodes.insert(up)
            {
                let _ = up;
            }
        }
    }
    let mut tap_nodes = HashSet::new();
    for id in &audio_nodes {
        if bridge_nodes.contains(id) {
            continue;
        }
        if is_rt_worker_sink(graph.node(*id).op_type.as_str()) {
            continue;
        }
        if graph.node(*id).flags.viewer {
            tap_nodes.insert(*id);
        }
    }
    let primary_sink = primary_rt_audio_sink(graph, &audio_nodes);
    let frame_cook_nodes = frame_cook_support(graph);
    AudioPartition {
        audio_nodes,
        bridge_nodes,
        tap_nodes,
        primary_sink,
        frame_cook_nodes,
        generation,
    }
}

fn frame_cook_support(graph: &Graph) -> HashSet<NodeId> {
    let mut need = HashSet::new();
    for id in graph.walk() {
        if !is_rt_audio_sink(graph.node(id).op_type.as_str()) {
            continue;
        }
        if is_rt_worker_sink(graph.node(id).op_type.as_str()) {
            continue;
        }
        walk_upstream_for_frame(graph, id, &mut need);
    }
    need
}

fn walk_upstream_for_frame(graph: &Graph, id: NodeId, out: &mut HashSet<NodeId>) {
    if !out.insert(id) {
        return;
    }
    for up in graph.node(id).inputs.iter().flatten() {
        walk_upstream_for_frame(graph, *up, out);
    }
}

fn walk_upstream_rt(
    graph: &Graph,
    id: NodeId,
    claimed: &mut HashSet<NodeId>,
    bridges: &mut HashSet<NodeId>,
) {
    let node = graph.node(id);
    if node.flags.bypass {
        return;
    }
    for input in &node.inputs {
        let Some(up) = *input else {
            continue;
        };
        if !rt_safe_chop(&graph.node(up).op_type) {
            bridges.insert(up);
            continue;
        }
        if claimed.insert(up) {
            walk_upstream_rt(graph, up, claimed, bridges);
        }
    }
}

fn rt_safe_chop(op: &str) -> bool {
    is_rt_safe_chop(op) || op == NULL || is_rt_worker_sink(op)
}

pub fn node_in_audio_domain(part: &AudioPartition, id: NodeId) -> bool {
    part.audio_nodes.contains(&id)
}

pub fn skip_frame_chop_cook(part: &AudioPartition, rt_armed: bool, graph: &Graph, id: NodeId) -> bool {
    if !rt_armed || !part.audio_nodes.contains(&id) {
        return false;
    }
    if part.bridge_nodes.contains(&id) || part.tap_nodes.contains(&id) {
        return false;
    }
    let op = graph.node(id).op_type.as_str();
    if matches!(op, AUDIO_IN | AUDIO_FILE | SPECTRUM | AUDIO_VST) {
        return false;
    }
    if is_rt_worker_sink(op) {
        return part.primary_sink == Some(id);
    }
    if (is_rt_safe_chop(op) || op == NULL) && part.frame_cook_nodes.contains(&id) {
        return false;
    }
    is_rt_safe_chop(op) || op == NULL
}