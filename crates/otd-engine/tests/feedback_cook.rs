use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry, Value};
use otd_engine::{Engines, registry};
use otd_gpu::GpuContext;
use otd_gpu::ops::FEEDBACK;

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
fn feedback_top_cooks_without_cycle_error() {
    let gpu = gpu_or_skip!();
    let reg = registry();
    let mut graph = Graph::new();
    let src = add(&mut graph, &reg, "constantTOP", "src");
    graph.set_param(src, "resw", Value::Int(16)).unwrap();
    graph.set_param(src, "resh", Value::Int(16)).unwrap();
    let target = add(&mut graph, &reg, "nullTOP", "target");
    graph.connect(src, target, 0).unwrap();
    let feedback = add(&mut graph, &reg, FEEDBACK, "fb1");
    graph
        .set_param(feedback, "target", Value::Str("/target".into()))
        .unwrap();
    let null = add(&mut graph, &reg, "nullTOP", "null1");
    graph.connect(feedback, null, 0).unwrap();

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let mut time = CookContext::default();
    for _ in 0..2 {
        engines.begin_frame();
        cook.cook_frame(&graph, &[null], &time, &mut engines)
            .unwrap();
        engines.end_frame(&graph);
        time.advance(time.dt);
    }
}
