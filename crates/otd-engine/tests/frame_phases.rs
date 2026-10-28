use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry, ParamEdit, Value};
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

fn add(graph: &mut Graph, reg: &OpRegistry, op: &str, name: &str) -> NodeId {
    let root = graph.root();
    let def = reg.get(op).unwrap().clone();
    graph.create(root, &def, Some(name)).unwrap()
}

#[test]
fn queued_edits_apply_before_the_next_cook() {
    let gpu = gpu_or_skip!();
    let reg = registry();
    let mut graph = Graph::new();
    let constant = add(&mut graph, &reg, "constantCHOP", "const1");
    graph.set_param(constant, "value0", Value::Float(1.0)).unwrap();
    let path = graph.path(constant);
    let edits = vec![ParamEdit::new(path, "value0", Value::Float(9.0))];
    let (n, problems) = otd_core::edit::apply(&mut graph, &edits);
    assert_eq!(n, 1, "{problems:?}");
    assert!(
        (graph.node(constant).param("value0").unwrap().value.as_f64() - 9.0).abs() < 1e-6
    );

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let time = CookContext::default();
    engines.begin_frame();
    cook.cook_frame(&graph, &[constant], &time, &mut engines)
        .unwrap();
    engines.end_frame();
}
