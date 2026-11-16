use otd_chop::io::{AUDIO_OUT, REAPER_OUT};
use otd_core::{Graph, OpRegistry};
use otd_engine::audio_partition::{partition_audio_domain, skip_frame_chop_cook};

fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    otd_chop::register_all(&mut r);
    r
}

#[test]
fn reaper_out_alone_stays_on_the_frame_cook() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let merge = graph
        .create(root, reg.get("mergeCHOP").unwrap(), Some("merge1"))
        .unwrap();
    let rep = graph
        .create(root, reg.get(REAPER_OUT).unwrap(), Some("reaper1"))
        .unwrap();
    graph.connect(merge, rep, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    assert!(part.audio_nodes.is_empty());
    assert!(part.primary_sink.is_none());
    assert!(!skip_frame_chop_cook(&part, true, &graph, merge));
    assert!(!skip_frame_chop_cook(&part, true, &graph, rep));
}

#[test]
fn secondary_reaper_out_still_cooks_on_the_frame_when_rt_owns_audio_out() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let merge = graph
        .create(root, reg.get("mergeCHOP").unwrap(), Some("merge1"))
        .unwrap();
    let audio = graph
        .create(root, reg.get(AUDIO_OUT).unwrap(), Some("audio1"))
        .unwrap();
    let rep = graph
        .create(root, reg.get(REAPER_OUT).unwrap(), Some("reaper1"))
        .unwrap();
    graph.connect(merge, audio, 0).unwrap();
    graph.connect(merge, rep, 0).unwrap();
    let part = partition_audio_domain(&graph, 1);
    assert_eq!(part.primary_sink, Some(audio));
    assert!(!skip_frame_chop_cook(&part, true, &graph, rep));
    assert!(skip_frame_chop_cook(&part, true, &graph, audio));
    assert!(part.frame_cook_nodes.contains(&merge));
    assert!(part.frame_cook_nodes.contains(&rep));
    assert!(!skip_frame_chop_cook(&part, true, &graph, merge));
}