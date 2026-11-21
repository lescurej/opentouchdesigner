use otd_audio::{AudioExecutor, AudioKernel, BlockCtx};
use otd_chop::data::{Channel, ChopData};
use otd_chop::io::AUDIO_OUT;
use otd_chop::rt::bridge;
use otd_chop::rt::plan_build::{build_audio_plan, plan_signature};
use otd_core::{Graph, NodeId, OpRegistry, Value};
use otd_engine::audio_partition::partition_audio_domain;
use std::sync::Arc;

fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    otd_chop::register_all(&mut r);
    r
}

fn add(graph: &mut Graph, reg: &OpRegistry, op: &str, name: &str) -> NodeId {
    let root = graph.root();
    graph
        .create(root, reg.get(op).unwrap(), Some(name))
        .unwrap()
}

fn no_counts(_: NodeId) -> Option<usize> {
    None
}

#[test]
fn diamond_chain_is_ordered_inputs_first() {
    let reg = registry();
    let mut graph = Graph::new();
    let osc = add(&mut graph, &reg, "audiooscillatorCHOP", "osc1");
    let a = add(&mut graph, &reg, "mathCHOP", "math1");
    let b = add(&mut graph, &reg, "mathCHOP", "math2");
    let merge = add(&mut graph, &reg, "mergeCHOP", "merge1");
    let out = add(&mut graph, &reg, AUDIO_OUT, "out1");
    graph.connect(osc, a, 0).unwrap();
    graph.connect(osc, b, 0).unwrap();
    graph.connect(a, merge, 0).unwrap();
    graph.connect(b, merge, 1).unwrap();
    graph.connect(merge, out, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    let (plan, build) = build_audio_plan(
        &graph,
        &part.sinks,
        &part.audio_nodes,
        &part.bridge_nodes,
        &no_counts,
    )
    .expect("plan");
    for (i, step) in plan.steps.iter().enumerate() {
        for input in &step.inputs {
            assert!(
                plan.steps[..i].iter().any(|s| s.output == *input),
                "step {i} reads slot {input} before it is written"
            );
        }
    }
    let merge_step = &plan.steps[plan.steps.len() - 2];
    assert_eq!(merge_step.inputs.len(), 2);
    assert_eq!(build.channels, 2, "merge of two mono chains is stereo");
}

#[test]
fn oscillator_plays_at_its_frequency_at_the_device_rate() {
    let reg = registry();
    let mut graph = Graph::new();
    let osc = add(&mut graph, &reg, "audiooscillatorCHOP", "osc1");
    let out = add(&mut graph, &reg, AUDIO_OUT, "out1");
    graph.connect(osc, out, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    let (mut plan, _) = build_audio_plan(
        &graph,
        &part.sinks,
        &part.audio_nodes,
        &part.bridge_nodes,
        &no_counts,
    )
    .expect("plan");
    let rate = 44_100.0;
    plan.prepare(rate);
    let mut ex = AudioExecutor::new(Box::new(plan), None, None);
    let mut out = vec![0.0f32; rate as usize];
    ex.render(&mut out, 1);
    let crossings = out
        .windows(2)
        .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
        .count();
    assert!(
        (crossings as i64 - 880).abs() <= 4,
        "440 Hz should cross zero ~880 times per second at 44.1 kHz, got {crossings}"
    );
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!((peak - 0.5).abs() < 0.02, "default amplitude, got {peak}");
}

#[test]
fn parameter_edits_do_not_change_the_plan_signature_but_rewiring_does() {
    let reg = registry();
    let mut graph = Graph::new();
    let osc = add(&mut graph, &reg, "audiooscillatorCHOP", "osc1");
    let math = add(&mut graph, &reg, "mathCHOP", "math1");
    let out = add(&mut graph, &reg, AUDIO_OUT, "out1");
    graph.connect(osc, out, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    let before = plan_signature(
        &graph,
        &part.sinks,
        &part.audio_nodes,
        &part.bridge_nodes,
        &no_counts,
    );
    if let Some(p) = graph.node_mut(osc).params.get_mut("frequency") {
        p.value = Value::Float(220.0);
    }
    let part = partition_audio_domain(&graph, 2);
    assert_eq!(
        before,
        plan_signature(
            &graph,
            &part.sinks,
            &part.audio_nodes,
            &part.bridge_nodes,
            &no_counts
        )
    );
    graph.connect(osc, math, 0).unwrap();
    graph.connect(math, out, 0).unwrap();
    let part = partition_audio_domain(&graph, 3);
    assert_ne!(
        before,
        plan_signature(
            &graph,
            &part.sinks,
            &part.audio_nodes,
            &part.bridge_nodes,
            &no_counts
        )
    );
}

#[test]
fn republished_graph_changes_the_sound_without_a_new_plan() {
    let reg = registry();
    let mut graph = Graph::new();
    let osc = add(&mut graph, &reg, "audiooscillatorCHOP", "osc1");
    let out = add(&mut graph, &reg, AUDIO_OUT, "out1");
    graph.connect(osc, out, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    let (mut plan, build) = build_audio_plan(
        &graph,
        &part.sinks,
        &part.audio_nodes,
        &part.bridge_nodes,
        &no_counts,
    )
    .expect("plan");
    plan.prepare(48_000.0);
    let mut ex = AudioExecutor::new(Box::new(plan), None, None);
    let mut buf = vec![0.0f32; 48_000];
    ex.render(&mut buf, 1);
    if let Some(p) = graph.node_mut(osc).params.get_mut("frequency") {
        p.value = Value::Float(220.0);
    }
    build
        .share
        .publish_graph(std::sync::Arc::new(graph.clone()));
    ex.render(&mut buf, 1);
    let crossings = buf
        .windows(2)
        .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
        .count();
    assert!((crossings as i64 - 440).abs() <= 4, "got {crossings}");
}

fn slice(start: f32, frames: usize) -> ChopData {
    let samples = (0..frames).map(|i| start + i as f32).collect();
    ChopData::new(vec![Channel::new("chan1", samples)], 48_000.0, true)
}

#[test]
fn bridge_streams_frame_slices_in_order_and_holds_on_underflow() {
    let (mut feed, mut kernel) = bridge(1, 4096);
    kernel.prepare(48_000.0, 4);
    let ctx = BlockCtx {
        sample_rate: 48_000.0,
        block_len: 4,
    };
    feed.publish(&Arc::new(slice(0.0, 6)));
    let mut out = [0.0f32; 4];
    kernel.process(&ctx, &[], &mut out);
    assert_eq!(out, [0.0, 1.0, 2.0, 3.0]);
    kernel.process(&ctx, &[], &mut out);
    assert_eq!(
        out,
        [4.0, 5.0, 5.0, 5.0],
        "underflow repeats the last frame"
    );
    feed.publish(&Arc::new(slice(6.0, 4)));
    kernel.process(&ctx, &[], &mut out);
    assert_eq!(out, [6.0, 7.0, 8.0, 9.0], "no slice is played twice");
}

#[test]
fn bridge_holds_a_constant_source() {
    let (mut feed, mut kernel) = bridge(2, 64);
    kernel.prepare(48_000.0, 4);
    let constant = ChopData::new(
        vec![Channel::new("a", vec![0.25]), Channel::new("b", vec![0.75])],
        60.0,
        false,
    );
    feed.publish(&Arc::new(constant));
    let mut out = [0.0f32; 8];
    kernel.process(
        &BlockCtx {
            sample_rate: 48_000.0,
            block_len: 4,
        },
        &[],
        &mut out,
    );
    assert_eq!(out, [0.25, 0.75, 0.25, 0.75, 0.25, 0.75, 0.25, 0.75]);
}

#[test]
fn reaper_out_alone_compiles_a_clock_plan() {
    let reg = registry();
    let mut graph = Graph::new();
    let osc = add(&mut graph, &reg, "audiooscillatorCHOP", "osc1");
    let rep = add(&mut graph, &reg, otd_chop::io::REAPER_OUT, "reaper1");
    graph.connect(osc, rep, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    let (plan, build) = build_audio_plan(
        &graph,
        &part.sinks,
        &part.audio_nodes,
        &part.bridge_nodes,
        &no_counts,
    )
    .expect("plan");
    assert!(
        build.device().is_none(),
        "no device: the worker runs its own clock"
    );
    assert!(plan.sink_slot.is_none());
    assert_eq!(build.sinks.len(), 1);
    assert_eq!(build.tap_node, osc);
}

#[test]
fn device_and_reaper_render_from_one_shared_chain() {
    let reg = registry();
    let mut graph = Graph::new();
    let osc = add(&mut graph, &reg, "audiooscillatorCHOP", "osc1");
    let out = add(&mut graph, &reg, AUDIO_OUT, "out1");
    let rep = add(&mut graph, &reg, otd_chop::io::REAPER_OUT, "reaper1");
    graph.connect(osc, out, 0).unwrap();
    graph.connect(osc, rep, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    let (plan, build) = build_audio_plan(
        &graph,
        &part.sinks,
        &part.audio_nodes,
        &part.bridge_nodes,
        &no_counts,
    )
    .expect("plan");
    assert_eq!(build.sinks.len(), 2);
    assert_eq!(
        plan.steps.len(),
        3,
        "the oscillator renders once for both sinks"
    );
    assert!(plan.sink_slot.is_some());
}

#[test]
fn bridge_hands_a_standalone_buffer_over_whole() {
    let (mut feed, mut kernel) = bridge(1, 64);
    kernel.prepare(48_000.0, 4);
    let table = Arc::new(ChopData::new(
        vec![Channel::new("chan1", (0..512).map(|i| i as f32).collect())],
        44_100.0,
        false,
    ));
    feed.publish(&table);
    let whole = feed.link().whole().expect("wavetable crosses whole");
    assert!(Arc::ptr_eq(&whole, &table));
    feed.publish(&Arc::new(slice(0.0, 6)));
    assert!(feed.link().whole().is_none(), "a stream is not a table");
}

#[test]
fn bridge_holds_a_control_rate_slice() {
    let (mut feed, mut kernel) = bridge(1, 64);
    kernel.prepare(48_000.0, 4);
    let control = ChopData::new(vec![Channel::new("chan1", vec![0.1, 0.2, 0.3])], 60.0, true);
    feed.publish(&Arc::new(control));
    let mut out = [0.0f32; 4];
    kernel.process(
        &BlockCtx {
            sample_rate: 48_000.0,
            block_len: 4,
        },
        &[],
        &mut out,
    );
    assert_eq!(out, [0.3; 4]);
}
