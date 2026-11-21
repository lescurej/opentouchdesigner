use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use otd_audio::{AudioKernel, AudioPlan, BLOCK_FRAMES, ControlPort, MAX_STEP_INPUTS, PlanStep};
use otd_core::{Graph, NodeId};

use crate::io::AUDIO_OUT;

use super::bridge::{BridgeFeed, BridgeLink, bridge};
use super::cook_kernel::{CookWrapKernel, KernelInput, SinkGainKernel, can_compile_rt_node};
use super::frame_share::FrameShare;
use super::reaper_sink::{ReaperFeed, ReaperSinkKernel};
use super::wiring::{rt_inputs, rt_source};

const BRIDGE_CAPACITY_FRAMES: usize = BLOCK_FRAMES * 64;

pub enum SinkKind {
    Device {
        volume: Arc<ControlPort>,
        device: String,
    },
    Reaper(ReaperFeed),
}

pub struct SinkBuild {
    pub id: NodeId,
    pub channels: usize,
    pub kind: SinkKind,
}

pub struct PlanBuild {
    pub sinks: Vec<SinkBuild>,
    /// Output layout of the clock: the device's, or the first stream's.
    pub channels: usize,
    pub tap_node: NodeId,
    pub tap_channels: usize,
    pub bridges: Vec<(NodeId, BridgeFeed)>,
    pub share: Arc<FrameShare>,
    produced: HashMap<NodeId, Arc<AtomicUsize>>,
}

impl PlanBuild {
    /// Channels the worker last cooked for `id`, once it has cooked a block.
    pub fn produced_channels(&self, id: NodeId) -> Option<usize> {
        self.produced
            .get(&id)
            .map(|c| c.load(Ordering::Relaxed))
            .filter(|c| *c > 0)
    }

    pub fn device(&self) -> Option<&str> {
        self.sinks.iter().find_map(|s| match &s.kind {
            SinkKind::Device { device, .. } => Some(device.as_str()),
            SinkKind::Reaper(_) => None,
        })
    }
}

/// Compiles every sink and the operators feeding them into one plan, so a
/// Reaper Out and an Audio Device Out share one clock and one render.
/// `channel_count` reports what the frame engine last cooked for a node, so
/// slots are sized to the real channel layout rather than a guess.
pub fn build_audio_plan(
    graph: &Graph,
    sinks: &[NodeId],
    audio_nodes: &HashSet<NodeId>,
    bridge_nodes: &HashSet<NodeId>,
    channel_count: &dyn Fn(NodeId) -> Option<usize>,
) -> Option<(AudioPlan, PlanBuild)> {
    let fed: Vec<(NodeId, NodeId)> = sinks
        .iter()
        .filter_map(|sink| rt_source(graph, *sink, 0).map(|src| (*sink, src)))
        .collect();
    let starts: Vec<NodeId> = fed.iter().map(|(_, src)| *src).collect();
    let order = topo_order(graph, &starts, audio_nodes, bridge_nodes)?;
    if !order
        .iter()
        .all(|id| bridge_nodes.contains(id) || can_compile_rt_node(graph, *id))
    {
        return None;
    }
    let share = FrameShare::new(Arc::new(graph.clone()));
    let mut kernels: Vec<Box<dyn AudioKernel>> = Vec::new();
    let mut steps = Vec::new();
    let mut slot_channels = Vec::new();
    let mut slot_for: HashMap<NodeId, usize> = HashMap::new();
    let mut links: HashMap<NodeId, BridgeLink> = HashMap::new();
    let mut bridges = Vec::new();
    let mut produced = HashMap::new();

    for id in order {
        let channels = channel_count(id)
            .filter(|c| *c > 0)
            .unwrap_or_else(|| channel_hint(graph, id))
            .max(1);
        let wired: Vec<(usize, NodeId, usize)> = rt_inputs(graph, id)
            .into_iter()
            .filter_map(|(pos, up)| slot_for.get(&up).map(|slot| (pos, up, *slot)))
            .take(MAX_STEP_INPUTS)
            .collect();
        let kernel: Box<dyn AudioKernel> = if bridge_nodes.contains(&id) {
            let (feed, kernel) = bridge(channels, BRIDGE_CAPACITY_FRAMES);
            links.insert(id, feed.link());
            bridges.push((id, feed));
            Box::new(kernel)
        } else {
            let wiring = wired
                .iter()
                .map(|(position, up, _)| KernelInput {
                    position: *position,
                    bridge: links.get(up).cloned(),
                })
                .collect();
            let count = Arc::new(AtomicUsize::new(0));
            produced.insert(id, count.clone());
            Box::new(CookWrapKernel::build(
                graph,
                id,
                share.clone(),
                wiring,
                count,
            )?)
        };
        let slot = slot_channels.len();
        slot_channels.push(channels);
        steps.push(PlanStep {
            kernel: kernels.len(),
            inputs: wired.iter().map(|(_, _, slot)| *slot).collect(),
            output: slot,
        });
        kernels.push(kernel);
        slot_for.insert(id, slot);
    }

    let mut device_slot = None;
    let mut built = Vec::new();
    for (sink, src) in &fed {
        let src_slot = *slot_for.get(src)?;
        let channels = slot_channels[src_slot];
        let node = graph.node(*sink);
        let (kernel, kind): (Box<dyn AudioKernel>, SinkKind) = if node.op_type == AUDIO_OUT {
            let volume = Arc::new(ControlPort::new(
                node.params
                    .get("volume")
                    .map(|p| p.value.as_f32())
                    .unwrap_or(1.0),
            ));
            let device = node
                .params
                .get("device")
                .map(|p| p.value.as_str().to_string())
                .unwrap_or_default();
            let gain = SinkGainKernel {
                volume: volume.clone(),
                channels,
            };
            (Box::new(gain), SinkKind::Device { volume, device })
        } else {
            let (feed, slot) = ReaperFeed::new();
            (
                Box::new(ReaperSinkKernel::new(slot, channels)),
                SinkKind::Reaper(feed),
            )
        };
        let out = slot_channels.len();
        slot_channels.push(channels);
        steps.push(PlanStep {
            kernel: kernels.len(),
            inputs: vec![src_slot],
            output: out,
        });
        kernels.push(kernel);
        if matches!(kind, SinkKind::Device { .. }) {
            device_slot.get_or_insert(out);
        }
        built.push(SinkBuild {
            id: *sink,
            channels,
            kind,
        });
    }

    let (_, tap_node) = *fed.first()?;
    let tap_slot = *slot_for.get(&tap_node)?;
    let tap_channels = slot_channels[tap_slot];
    let mut plan = AudioPlan::new(BLOCK_FRAMES, slot_channels, kernels, steps);
    plan.sink_slot = device_slot;
    plan.tap_slot = Some(tap_slot);
    let channels = built.first().map(|s| s.channels).unwrap_or(tap_channels);
    Some((
        plan,
        PlanBuild {
            sinks: built,
            channels,
            tap_node,
            tap_channels,
            bridges,
            share,
            produced,
        },
    ))
}

/// Changes only when the compiled plan would differ: operators, wiring,
/// channel layouts and the output device. Parameter edits do not recompile;
/// the kernels read them from the republished graph.
pub fn plan_signature(
    graph: &Graph,
    sinks: &[NodeId],
    audio_nodes: &HashSet<NodeId>,
    bridge_nodes: &HashSet<NodeId>,
    channel_count: &dyn Fn(NodeId) -> Option<usize>,
) -> u64 {
    let mut h = DefaultHasher::new();
    sinks.hash(&mut h);
    let mut ids: Vec<NodeId> = audio_nodes.iter().chain(bridge_nodes).copied().collect();
    ids.sort();
    ids.dedup();
    for id in ids {
        let Some(node) = graph.get(id) else {
            continue;
        };
        id.hash(&mut h);
        node.op_type.hash(&mut h);
        bridge_nodes.contains(&id).hash(&mut h);
        rt_inputs(graph, id).hash(&mut h);
        channel_count(id).filter(|c| *c > 0).hash(&mut h);
        if node.op_type == AUDIO_OUT
            && let Some(p) = node.params.get("device")
        {
            p.value.as_str().hash(&mut h);
        }
    }
    h.finish()
}

/// Post-order DFS: every node comes after all of its inputs. Bridges are
/// leaves — their upstream belongs to the frame cook.
fn topo_order(
    graph: &Graph,
    starts: &[NodeId],
    audio_nodes: &HashSet<NodeId>,
    bridge_nodes: &HashSet<NodeId>,
) -> Option<Vec<NodeId>> {
    let mut order = Vec::new();
    let mut done = HashSet::new();
    let mut on_path = HashSet::new();
    let mut stack: Vec<(NodeId, bool)> = starts.iter().rev().map(|s| (*s, false)).collect();
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
        for (_, up) in rt_inputs(graph, id).into_iter().rev() {
            if !done.contains(&up) {
                stack.push((up, false));
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
    let ups = rt_inputs(graph, id);
    match node.op_type.as_str() {
        "mergeCHOP" => ups
            .iter()
            .map(|(_, up)| channel_hint(graph, *up))
            .sum::<usize>()
            .max(1),
        "audiooscillatorCHOP" => 1,
        _ => ups
            .first()
            .map(|(_, up)| channel_hint(graph, *up))
            .unwrap_or(1),
    }
}
