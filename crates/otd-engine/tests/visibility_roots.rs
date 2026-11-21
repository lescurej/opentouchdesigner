use otd_chop::io::AUDIO_OUT;
use otd_core::{Graph, OpRegistry};
use otd_engine::{
    CookRootsRequest, EditorRootsRequest, OutputRootsRequest, app_cook_roots, editor_roots,
    output_roots,
};

fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    otd_chop::register_all(&mut r);
    otd_gpu::register_all(&mut r);
    r
}

#[test]
fn output_roots_stable_without_editor_viewers() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let def = reg.get(AUDIO_OUT).unwrap();
    let audio = graph.create(root, def, Some("audioout1")).unwrap();
    graph.node_mut(audio).flags.viewer = false;
    let const_def = reg.get("constantCHOP").unwrap();
    let viewer = graph.create(root, const_def, Some("c1")).unwrap();
    graph.node_mut(viewer).flags.viewer = true;

    let out_only = output_roots(OutputRootsRequest {
        graph: &graph,
        perform_ui: None,
        extra_roots: &[],
    });
    assert!(out_only.contains(&audio));

    let merged = app_cook_roots(CookRootsRequest {
        graph: &graph,
        perform_ui: None,
        visible: &[viewer],
        current: root,
        output_extra_roots: &[],
        editor_extra_roots: &[],
        include_editor: false,
    });
    assert!(!merged.contains(&viewer));

    let with_editor = app_cook_roots(CookRootsRequest {
        graph: &graph,
        perform_ui: None,
        visible: &[viewer],
        current: root,
        output_extra_roots: &[],
        editor_extra_roots: &[],
        include_editor: true,
    });
    assert!(with_editor.contains(&viewer));
    assert_eq!(
        editor_roots(EditorRootsRequest {
            graph: &graph,
            visible: &[viewer],
            extra_roots: &[],
        }),
        vec![viewer]
    );
}
