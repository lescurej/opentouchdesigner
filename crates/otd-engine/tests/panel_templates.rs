use otd_core::{Graph, OpRegistry, Value};
use otd_engine::{panel, panel_templates, registry};

fn add(graph: &mut Graph, reg: &OpRegistry, parent: otd_core::NodeId, op: &str, name: &str) -> otd_core::NodeId {
    let def = reg.get(op).unwrap().clone();
    graph.create(parent, &def, Some(name)).unwrap()
}

fn child_names(graph: &Graph, comp: otd_core::NodeId) -> Vec<String> {
    let mut names: Vec<String> = graph
        .children(comp)
        .iter()
        .map(|id| graph.node(*id).name.clone())
        .collect();
    names.sort();
    names
}

#[test]
fn button_comp_gets_td_style_internals() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let btn = add(&mut graph, &reg, root, panel::BUTTON, "button1");
    panel_templates::install_button(&mut graph, &reg, btn);

    let names = child_names(&graph, btn);
    for need in ["text", "view1", "panel1", "bg1", "par1", "out1"] {
        assert!(names.iter().any(|n| n == need), "button missing `{need}`: {names:?}");
    }
    assert_eq!(
        graph.node(btn).params["label"].value,
        Value::Str("Button".into())
    );
}

#[test]
fn slider_comp_gets_td_style_internals() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let s = add(&mut graph, &reg, root, panel::SLIDER, "slider1");
    panel_templates::install_slider(&mut graph, &reg, s);

    let names = child_names(&graph, s);
    for need in ["text", "view1", "panel1", "track1", "fill1", "par1", "out1"] {
        assert!(names.iter().any(|n| n == need), "slider missing `{need}`: {names:?}");
    }
}
