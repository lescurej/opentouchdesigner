use otd_core::{Connector, Family, Graph, NodeId};

use crate::ops::{is_rt_audio_sink, is_rt_device_sink, is_rt_safe_chop};

const MAX_HOPS: usize = 64;

/// Every terminal the audio worker drives, clock owner first: the first
/// Audio Device Out in walk order, then every Reaper Out.
pub fn rt_sinks(graph: &Graph) -> Vec<NodeId> {
    let mut device = None;
    let mut streams = Vec::new();
    for id in graph.walk() {
        let node = graph.node(id);
        if node.flags.bypass {
            continue;
        }
        let op = node.op_type.as_str();
        if is_rt_device_sink(op) {
            device.get_or_insert(id);
        } else if is_rt_audio_sink(op) {
            streams.push(id);
        }
    }
    device.into_iter().chain(streams).collect()
}

/// The operator that actually produces what `id` sees on input `index`:
/// bypass, component boundaries, In/Out connectors and pass-through sinks are
/// looked through, so a chain inside a component compiles like a flat one.
pub fn rt_source(graph: &Graph, id: NodeId, index: usize) -> Option<NodeId> {
    let mut cur = graph.resolve_input_source(id, index)?;
    for _ in 0..MAX_HOPS {
        let node = graph.get(cur)?;
        cur = match node.connector {
            Connector::In => graph.connector_source(cur)?,
            Connector::Out if node.family == Family::Chop => graph.resolve_input_source(cur, 0)?,
            _ if is_rt_audio_sink(&node.op_type) => graph.resolve_input_source(cur, 0)?,
            _ => return Some(cur),
        };
    }
    None
}

/// `(input position, producer)` for every connected input, positions kept so
/// an operator with a gap in its inputs reads each one where it was wired.
pub fn rt_inputs(graph: &Graph, id: NodeId) -> Vec<(usize, NodeId)> {
    let node = graph.node(id);
    (0..node.inputs.len())
        .filter(|i| node.inputs[*i].is_some())
        .filter_map(|i| rt_source(graph, id, i).map(|src| (i, src)))
        .collect()
}

/// Whether the worker runs `op` itself instead of receiving it from the frame cook.
pub fn runs_on_worker(graph: &Graph, id: NodeId) -> bool {
    let node = graph.node(id);
    node.family == Family::Chop
        && is_rt_safe_chop(&node.op_type)
        && !is_rt_audio_sink(&node.op_type)
}
