use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry, Value};
use otd_engine::{Engines, registry};
use otd_gpu::GpuContext;

macro_rules! gpu_or_skip {
    () => {
        match GpuContext::headless() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skipping: no GPU available ({e})");
                return;
            }
        }
    };
}

fn add(graph: &mut Graph, reg: &OpRegistry, parent: NodeId, op: &str, name: &str) -> NodeId {
    let def = reg.get(op).unwrap().clone();
    graph.create(parent, &def, Some(name)).unwrap()
}

#[test]
fn select_chop_resolves_chop_param_star_slash_dmx() {
    let gpu = gpu_or_skip!();
    let reg = registry();
    let mut graph = Graph::new();
    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let time = CookContext::default();

    let root = graph.root();
    let comp = add(&mut graph, &reg, root, "baseCOMP", "comp");
    let inner = add(&mut graph, &reg, comp, "baseCOMP", "inner");
    let dmx = add(&mut graph, &reg, comp, "constantCHOP", "dmx");
    graph.set_param(dmx, "value0", Value::Float(42.0)).unwrap();
    let sel = add(&mut graph, &reg, inner, "selectCHOP", "sel");
    graph
        .set_param(sel, "chop", Value::Str("*/dmx".into()))
        .unwrap();

    engines.begin_frame();
    cook.cook_frame(&graph, &[sel], &time, &mut engines)
        .unwrap();
    engines.end_frame(&graph);

    let data = engines.chop_data(sel).expect("select cooked");
    assert_eq!(data.value("chan1"), Some(42.0));
}
