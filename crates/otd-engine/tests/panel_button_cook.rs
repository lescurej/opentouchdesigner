use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry, Value};
use otd_engine::{panel, panel_templates, registry, Engines};
use otd_gpu::{read_pixels_rgba8, GpuContext};

fn gpu_or_skip() -> Option<GpuContext> {
    match GpuContext::headless() {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("skipping: no GPU ({e})");
            None
        }
    }
}

fn add(graph: &mut Graph, reg: &OpRegistry, parent: NodeId, op: &str, name: &str) -> NodeId {
    let def = reg.get(op).unwrap().clone();
    graph.create(parent, &def, Some(name)).unwrap()
}

fn text_top_under_button(graph: &Graph, btn: NodeId) -> NodeId {
    let text = graph
        .children(btn)
        .iter()
        .find(|id| graph.node(**id).name == "text")
        .copied()
        .expect("textCOMP");
    graph
        .children(text)
        .iter()
        .find(|id| graph.node(**id).op_type == "textTOP")
        .copied()
        .expect("textTOP")
}

#[test]
fn button_label_reaches_interior_text_top() {
    let Some(gpu) = gpu_or_skip() else {
        return;
    };
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let btn = add(&mut graph, &reg, root, panel::BUTTON, "button1");
    panel_templates::install_button(&mut graph, &reg, btn);
    graph
        .set_param(btn, "label", Value::Str("Go".into()))
        .unwrap();

    let top = text_top_under_button(&graph, btn);

    let mut engines = Engines::new(gpu.clone());
    let mut cook = CookEngine::new();
    let time = CookContext::default();
    engines.begin_frame();
    cook.cook_frame(&graph, &[top], &time, &mut engines)
        .unwrap();
    engines.end_frame();

    let tex = engines.top.output(&graph, top).unwrap().clone();
    let px = read_pixels_rgba8(&gpu, &tex).unwrap().2;
    let ink = px.chunks(4).filter(|p| p[3] > 8).count();
    assert!(ink > 40, "button label should rasterize, got {ink} ink pixels");
}
