use super::*;
use otd_chop::io::AUDIO_OUT;
use otd_core::OpRegistry;

fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    otd_chop::register_all(&mut r);
    r
}

#[test]
fn audio_device_out_is_a_root_without_the_viewer_flag() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let def = reg.get(AUDIO_OUT).unwrap();
    let id = graph.create(root, def, Some("audioout1")).unwrap();
    graph.node_mut(id).flags.viewer = false;

    let roots = editor_cook_roots(&graph, None);
    assert!(
        roots.contains(&id),
        "audiodeviceout must keep cooking when viewer is off"
    );
}

#[test]
fn output_roots_ignore_canvas_viewers() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let def = reg.get("constantCHOP").unwrap();
    let id = graph.create(root, def, Some("const1")).unwrap();
    graph.node_mut(id).flags.viewer = true;
    let roots = output_roots(OutputRootsRequest {
        graph: &graph,
        perform_ui: None,
        extra_roots: &[],
    });
    assert!(!roots.contains(&id));
}

#[test]
fn editor_roots_include_visible_viewers() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let def = reg.get("constantCHOP").unwrap();
    let id = graph.create(root, def, Some("const1")).unwrap();
    graph.node_mut(id).flags.viewer = true;
    let roots = editor_roots(EditorRootsRequest {
        graph: &graph,
        visible: &[id],
        extra_roots: &[],
    });
    assert!(roots.contains(&id));
}
