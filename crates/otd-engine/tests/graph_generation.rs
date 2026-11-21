use otd_chop::io::AUDIO_OUT;
use otd_core::{Graph, OpRegistry};

fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    otd_chop::register_all(&mut r);
    r
}

#[test]
fn topology_generation_changes_when_wired() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let before = graph.audio_topology_generation();
    let osc = reg.get("audiooscillatorCHOP").unwrap();
    let out = reg.get(AUDIO_OUT).unwrap();
    let o = graph.create(root, osc, Some("osc1")).unwrap();
    let d = graph.create(root, out, Some("out1")).unwrap();
    graph.connect(o, d, 0).unwrap();
    let after = graph.audio_topology_generation();
    assert_ne!(before, after);
}
