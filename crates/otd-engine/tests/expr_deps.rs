use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry, Param, ParamMode, Value};
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
fn relative_op_expression_cooks_source_first() {
    let gpu = gpu_or_skip!();
    let reg = registry();
    let mut graph = Graph::new();
    let lfo = add(&mut graph, &reg, "lfoCHOP", "lfo1");
    graph
        .set_param(lfo, "frequency", Value::Float(1.0))
        .unwrap();
    let blur = add(&mut graph, &reg, "blurTOP", "blur1");
    graph
        .set_expression(blur, "size", "op('lfo1')['chan1']")
        .unwrap();
    assert_eq!(
        graph.node(blur).param("size").unwrap().mode,
        ParamMode::Expression
    );

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let time = CookContext::default();
    engines.begin_frame();
    cook.cook_frame(&graph, &[blur, lfo], &time, &mut engines)
        .unwrap();
    engines.end_frame();

    let v = engines
        .eval_param(&graph, blur, "size", &time)
        .unwrap()
        .as_f64();
    assert!(v.is_finite());
}

#[test]
fn comp_local_rate_scales_time_in_expressions() {
    let gpu = gpu_or_skip!();
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let comp = graph
        .create(
            root,
            &reg.get("containerCOMP").unwrap().clone(),
            Some("comp1"),
        )
        .unwrap();
    graph.add_custom_param(comp, "rate", Param::new(Value::Float(2.0)));
    let lfo_def = reg.get("lfoCHOP").unwrap().clone();
    let lfo = graph.create(comp, &lfo_def, Some("lfo1")).unwrap();
    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let time = CookContext {
        time: 1.0,
        abs_time: 1.0,
        ..Default::default()
    };
    engines.begin_frame();
    cook.cook_frame(&graph, &[lfo], &time, &mut engines)
        .unwrap();
    engines.end_frame();
    let (local, _, _) = graph.local_clock_at(lfo, &time);
    assert!((local - 2.0).abs() < 1e-6);
}
