use otd_chop::io::{AUDIO_OUT, REAPER_OUT};
use otd_chop::rt::wiring::rt_inputs;
use otd_core::{Graph, NodeId, OpRegistry};
use otd_engine::audio_partition::{partition_audio_domain, skip_frame_chop_cook};

fn add(graph: &mut Graph, reg: &OpRegistry, parent: NodeId, op: &str, name: &str) -> NodeId {
    let id = graph
        .create(parent, reg.get(op).unwrap(), Some(name))
        .unwrap();
    graph.node_mut(id).flags.viewer = false;
    id
}

#[test]
fn reaper_out_runs_on_the_audio_clock() {
    let reg = otd_engine::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let osc = add(&mut graph, &reg, root, "audiooscillatorCHOP", "osc1");
    let rep = add(&mut graph, &reg, root, REAPER_OUT, "reaper1");
    graph.connect(osc, rep, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    assert_eq!(part.sinks, vec![rep]);
    assert!(part.audio_nodes.contains(&osc));
    assert!(skip_frame_chop_cook(&part, true, osc));
    assert!(skip_frame_chop_cook(&part, true, rep));
    assert!(
        !skip_frame_chop_cook(&part, false, rep),
        "frame fallback when the worker is down"
    );
}

#[test]
fn reaper_out_shares_the_device_clock() {
    let reg = otd_engine::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let merge = add(&mut graph, &reg, root, "mergeCHOP", "merge1");
    let rep = add(&mut graph, &reg, root, REAPER_OUT, "reaper1");
    let audio = add(&mut graph, &reg, root, AUDIO_OUT, "audio1");
    graph.connect(merge, audio, 0).unwrap();
    graph.connect(merge, rep, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    assert_eq!(part.sinks, vec![audio, rep], "device owns the clock");
    assert!(skip_frame_chop_cook(&part, true, merge));
}

#[test]
fn reaper_out_reaches_inside_a_component() {
    let reg = otd_engine::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let osc = add(&mut graph, &reg, root, "audiooscillatorCHOP", "osc1");
    let comp = add(&mut graph, &reg, root, "baseCOMP", "fx");
    let inside_in = add(&mut graph, &reg, comp, "inCHOP", "in1");
    let math = add(&mut graph, &reg, comp, "mathCHOP", "math1");
    let inside_out = add(&mut graph, &reg, comp, "outCHOP", "out1");
    graph.sync_connectors(comp);
    graph.connect(inside_in, math, 0).unwrap();
    graph.connect(math, inside_out, 0).unwrap();
    graph.connect(osc, comp, 0).unwrap();
    let rep = add(&mut graph, &reg, root, REAPER_OUT, "reaper1");
    graph.connect(comp, rep, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    assert!(part.audio_nodes.contains(&math));
    assert!(part.audio_nodes.contains(&osc));
    assert!(
        part.bridge_nodes.is_empty(),
        "the whole chain renders on the worker"
    );
    assert!(skip_frame_chop_cook(&part, true, math));
}

#[test]
fn a_frame_reader_keeps_its_source_frame_cooked() {
    let reg = otd_engine::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let osc = add(&mut graph, &reg, root, "audiooscillatorCHOP", "osc1");
    let rep = add(&mut graph, &reg, root, REAPER_OUT, "reaper1");
    let analyze = add(&mut graph, &reg, root, "analyzeCHOP", "analyze1");
    graph.connect(osc, rep, 0).unwrap();
    graph.connect(osc, analyze, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    assert!(part.audio_nodes.contains(&osc));
    assert!(!skip_frame_chop_cook(&part, true, osc));
}

#[test]
fn a_viewed_operator_keeps_frame_cooking_for_its_viewer() {
    let reg = otd_engine::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let osc = add(&mut graph, &reg, root, "audiooscillatorCHOP", "osc1");
    let rep = add(&mut graph, &reg, root, REAPER_OUT, "reaper1");
    graph.connect(osc, rep, 0).unwrap();
    graph.node_mut(osc).flags.viewer = true;
    graph.node_mut(rep).flags.viewer = true;
    let part = partition_audio_domain(&graph, 1);
    assert!(!skip_frame_chop_cook(&part, true, osc));
    assert!(
        skip_frame_chop_cook(&part, true, rep),
        "a sink never sends from the frame"
    );
}

#[test]
fn gapped_inputs_keep_their_positions() {
    let reg = otd_engine::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let osc = add(&mut graph, &reg, root, "audiooscillatorCHOP", "osc1");
    let merge = add(&mut graph, &reg, root, "mergeCHOP", "merge1");
    graph.connect(osc, merge, 1).unwrap();
    assert_eq!(rt_inputs(&graph, merge), vec![(1, osc)]);
}
