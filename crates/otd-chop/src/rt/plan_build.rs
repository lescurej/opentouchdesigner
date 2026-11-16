use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use otd_audio::{AudioKernel, AudioPlan, ControlPort, PlanStep, BLOCK_FRAMES, MAX_STEP_INPUTS};
use otd_core::{Graph, NodeId};

use crate::io::AUDIO_OUT;
use crate::ops::primary_rt_audio_sink;

use super::bridge::{bridge, BridgeFeed};
use super::cook_kernel::{can_compile_rt_node, CookWrapKernel, SinkGainKernel};
use super::frame_share::FrameShare;
use super::reaper_sink::{ReaperSinkKernel, ReaperStreamSlot};

const BRIDGE_CAPACITY_FRAMES: usize = BLOCK_FRAMES * 32;

pub struct PlanBuild {
    pub sink_id: NodeId,
    pub sink_is_device: bool,
    pub device: String,
    pub channels: usize,
    pub tap_node: NodeId,
    pub tap_channels: usize,
    pub bridges: Vec<(NodeId, BridgeFeed)>,
    pub share: Arc<FrameShare>,
    pub sink_volume: Option<Arc<ControlPort>>,
    pub reaper_slot: Option<Arc<Mutex<ReaperStreamSlot>>>,
}

/// Compiles the chain feeding the primary audio sink. `channel_count` reports
/// what the frame engine last cooked for a node, so slots are sized to the
/// real channel layout rather than a guess.
pub fn build_audio_plan(
    graph: &Graph,
    audio_nodes: &HashSet<NodeId>,
    bridge_nodes: &HashSet<NodeId>,
    channel_count: &dyn Fn(NodeId) -> Option<usize>,
) -> Option<(AudioPlan, PlanBuild)> {
    if !chain_compiles(graph, audio_nodes, bridge_nodes) {
        return None;
    }
    let sink = primary_rt_audio_sink(graph, audio_nodes)?;
    let wired = graph.node(sink).inputs.first().copied().flatten()?;
    let order = topo_order(graph, wired, audio_nodes, bridge_nodes)?;
    let share = FrameShare::new(Arc::new(graph.clone()));
    let mut kernels: Vec<Box<dyn AudioKernel>> = Vec::new();
    let mut steps = Vec::new();
    let mut slot_channels = Vec::new();
    let mut slot_for: HashMap<NodeId, usize> = HashMap::new();
    let mut bridges = Vec::new();

    for id in order {
        let channels = channel_count(id)
            .filter(|c| *c > 0)
            .unwrap_or_else(|| channel_hint(graph, id))
            .max(1);
        let inputs: Vec<usize> = graph
            .node(id)
            .inputs
            .iter()
            .flatten()
            .filter_map(|up| slot_for.get(up).copied())
            .take(MAX_STEP_INPUTS)
            .collect();
        let kernel: Box<dyn AudioKernel> = if bridge_nodes.contains(&id) {
            let (feed, kernel) = bridge(channels, BRIDGE_CAPACITY_FRAMES);
            bridges.push((id, feed));
            Box::new(kernel)
        } else {
            Box::new(CookWrapKernel::build(graph, id, share.clone())?)
        };
        let slot = slot_channels.len();
        slot_channels.push(channels);
        steps.push(PlanStep {
            kernel: kernels.len(),
            inputs,
            output: slot,
        });
        kernels.push(kernel);
        slot_for.insert(id, slot);
    }

    let tap_slot = *slot_for.get(&wired)?;
    let channels = slot_channels[tap_slot];
    let sink_node = graph.node(sink);
    let sink_is_device = sink_node.op_type == AUDIO_OUT;
    let (sink_kernel, sink_volume, reaper_slot): (Box<dyn AudioKernel>, _, _) = if sink_is_device {
        let volume = Arc::new(ControlPort::new(
            sink_node.params.get("volume").map(|p| p.value.as_f32()).unwrap_or(1.0),
        ));
        let gain = SinkGainKernel {
            volume: volume.clone(),
            channels,
        };
        (Box::new(gain), Some(volume), None)
    } else {
        let slot = Arc::new(Mutex::new(ReaperStreamSlot {
            active: true,
            ..Default::default()
        }));
        (Box::new(ReaperSinkKernel::new(slot.clone(), channels)), None, Some(slot))
    };
    let sink_slot = slot_channels.len();
    slot_channels.push(channels);
    steps.push(PlanStep {
        kernel: kernels.len(),
        inputs: vec![tap_slot],
        output: sink_slot,
    });
    kernels.push(sink_kernel);

    let mut plan = AudioPlan::new(BLOCK_FRAMES, slot_channels, kernels, steps);
    plan.sink_slot = Some(sink_slot);
    plan.tap_slot = Some(tap_slot);
    let device = sink_node
        .params
        .get("device")
        .map(|p| p.value.as_str().to_string())
        .unwrap_or_default();
    Some((
        plan,
        PlanBuild {
            sink_id: sink,
            sink_is_device,
            device,
            channels,
            tap_node: wired,
            tap_channels: channels,
            bridges,
            share,
            sink_volume,
            reaper_slot,
        },
    ))
}

/// Changes only when the compiled plan would differ: operators, wiring and
/// the output device. Parameter edits do not recompile; the kernels read them
/// from the republished graph.
pub fn plan_signature(graph: &Graph, audio_nodes: &HashSet<NodeId>, bridge_nodes: &HashSet<NodeId>) -> u64 {
    let mut h = DefaultHasher::new();
    let mut ids: Vec<NodeId> = audio_nodes.iter().chain(bridge_nodes).copied().collect();
    ids.sort();
    ids.dedup();
    for id in ids {
        let Some(node) = graph.get(id) else {
            continue;
        };
        id.hash(&mut h);
        node.op_type.hash(&mut h);
        node.flags.bypass.hash(&mut h);
        node.inputs.hash(&mut h);
        if node.op_type == AUDIO_OUT
            && let Some(p) = node.params.get("device")
        {
            p.value.as_str().hash(&mut h);
        }
    }
    h.finish()
}

pub fn chain_compiles(graph: &Graph, audio_nodes: &HashSet<NodeId>, bridge_nodes: &HashSet<NodeId>) -> bool {
    let Some(sink) = primary_rt_audio_sink(graph, audio_nodes) else {
        return false;
    };
    let Some(wired) = graph.node(sink).inputs.first().copied().flatten() else {
        return false;
    };
    let Some(order) = topo_order(graph, wired, audio_nodes, bridge_nodes) else {
        return false;
    };
    order
        .iter()
        .all(|id| bridge_nodes.contains(id) || can_compile_rt_node(graph, *id))
}

/// Post-order DFS: every node comes after all of its inputs. Bridges are
/// leaves — their upstream belongs to the frame cook.
fn topo_order(
    graph: &Graph,
    start: NodeId,
    audio_nodes: &HashSet<NodeId>,
    bridge_nodes: &HashSet<NodeId>,
) -> Option<Vec<NodeId>> {
    let mut order = Vec::new();
    let mut done = HashSet::new();
    let mut on_path = HashSet::new();
    let mut stack = vec![(start, false)];
    while let Some((id, expanded)) = stack.pop() {
        if expanded {
            on_path.remove(&id);
            if done.insert(id) {
                order.push(id);
            }
            continue;
        }
        if done.contains(&id) {
            continue;
        }
        let is_bridge = bridge_nodes.contains(&id);
        if !is_bridge && !audio_nodes.contains(&id) {
            return None;
        }
        if !on_path.insert(id) {
            return None;
        }
        stack.push((id, true));
        if is_bridge {
            continue;
        }
        for up in graph.node(id).inputs.iter().flatten() {
            if !done.contains(up) {
                stack.push((*up, false));
            }
        }
    }
    Some(order)
}

fn channel_hint(graph: &Graph, id: NodeId) -> usize {
    let node = graph.node(id);
    if let Some(p) = node.params.get("channels") {
        return (p.value.as_i64().max(1)) as usize;
    }
    match node.op_type.as_str() {
        "mergeCHOP" => node
            .inputs
            .iter()
            .flatten()
            .map(|up| channel_hint(graph, *up))
            .sum::<usize>()
            .max(1),
        "audiooscillatorCHOP" => 1,
        _ => node
            .inputs
            .iter()
            .flatten()
            .next()
            .map(|up| channel_hint(graph, *up))
            .unwrap_or(1),
    }
}
