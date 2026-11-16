use otd_chop::io::AUDIO_OUT;
use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry};
use otd_engine::{
    app_cook_roots, output_roots, registry, CookRootsRequest, Engines, OutputRootsRequest,
};
use otd_gpu::GpuContext;

fn chop_registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    otd_chop::register_all(&mut r);
    r
}

fn add(graph: &mut Graph, reg: &OpRegistry, op: &str, name: &str) -> NodeId {
    let root = graph.root();
    let def = reg.get(op).unwrap().clone();
    graph.create(root, &def, Some(name)).unwrap()
}

#[test]
fn hidden_playback_excludes_editor_viewer_roots() {
    let reg = chop_registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let o = add(&mut graph, &reg, "audiooscillatorCHOP", "osc1");
    let d = add(&mut graph, &reg, AUDIO_OUT, "out1");
    let viewer = add(&mut graph, &reg, "constantCHOP", "c1");
    graph.connect(o, d, 0).unwrap();
    graph.node_mut(d).flags.render = true;
    graph.node_mut(viewer).flags.viewer = true;

    let output_only = output_roots(OutputRootsRequest {
        graph: &graph,
        perform_ui: None,
        extra_roots: &[],
    });
    let roots = |include_editor| {
        app_cook_roots(CookRootsRequest {
            graph: &graph,
            perform_ui: None,
            visible: &[viewer],
            current: root,
            output_extra_roots: &[],
            editor_extra_roots: &[],
            include_editor,
        })
    };
    let hidden = roots(false);
    let visible = roots(true);

    assert!(output_only.contains(&d));
    assert!(!hidden.contains(&viewer));
    assert!(visible.contains(&viewer));
    assert!(visible.contains(&d));
    assert_eq!(hidden, output_only);
}

fn cook_frames(graph: &Graph, roots: &[NodeId], watch: NodeId, frames: i64) -> Option<Vec<Vec<f32>>> {
    let gpu = GpuContext::headless().ok()?;
    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let mut out = Vec::new();
    let mut time = CookContext::default();
    for _ in 0..frames {
        engines.begin_frame();
        cook.cook_frame(graph, roots, &time, &mut engines).unwrap();
        engines.end_frame(graph);
        let data = engines.channels.get(watch)?;
        out.push(data.channels.iter().flat_map(|c| c.samples.clone()).collect());
        time.advance(time.dt);
    }
    Some(out)
}

#[test]
fn hidden_cook_produces_the_same_output_channels_as_visible_cook() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let lfo = add(&mut graph, &reg, "lfoCHOP", "lfo1");
    let lag = add(&mut graph, &reg, "lagCHOP", "lag1");
    let viewer = add(&mut graph, &reg, "nullCHOP", "view1");
    graph.connect(lfo, lag, 0).unwrap();
    graph.connect(lag, viewer, 0).unwrap();
    graph.node_mut(viewer).flags.viewer = true;
    let roots = |include_editor| {
        app_cook_roots(CookRootsRequest {
            graph: &graph,
            perform_ui: None,
            visible: &[viewer],
            current: root,
            output_extra_roots: &[lag],
            editor_extra_roots: &[],
            include_editor,
        })
    };
    let (hidden, visible) = (roots(false), roots(true));
    assert!(!hidden.contains(&viewer) && visible.contains(&viewer));
    let Some(a) = cook_frames(&graph, &hidden, lag, 120) else {
        eprintln!("skipping: no GPU available");
        return;
    };
    let b = cook_frames(&graph, &visible, lag, 120).expect("second run");
    assert_eq!(a, b, "viewer cooks must not change what the output sees");
}
