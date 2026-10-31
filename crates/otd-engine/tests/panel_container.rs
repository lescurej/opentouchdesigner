use otd_core::{Graph, OpRegistry, Value};
use otd_engine::{panel, registry};

fn add(
    graph: &mut Graph,
    reg: &OpRegistry,
    parent: otd_core::NodeId,
    op: &str,
    name: &str,
) -> otd_core::NodeId {
    let def = reg.get(op).unwrap().clone();
    graph.create(parent, &def, Some(name)).unwrap()
}

#[test]
fn container_display_off_hides_child_widgets() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let container = add(&mut graph, &reg, root, panel::CONTAINER, "ui");
    let btn = add(&mut graph, &reg, container, panel::BUTTON, "btn1");
    graph
        .set_param(container, "display", Value::Bool(false))
        .unwrap();

    let widgets = panel::widgets(&graph);
    assert!(
        !widgets.iter().any(|w| w.id == btn),
        "button inside hidden container must not appear in panel"
    );
    assert!(
        !widgets.iter().any(|w| w.id == container),
        "hidden container must not appear"
    );
}

#[test]
fn container_has_td_layout_and_children_params() {
    let reg = registry();
    let keys: Vec<String> = (reg.get(panel::CONTAINER).unwrap().params)()
        .keys()
        .cloned()
        .collect();
    for key in [
        "vieww",
        "viewh",
        "hmode",
        "vmode",
        "leftanchor",
        "topanchor",
        "layer",
        "alignmax",
        "justifyh",
        "composite",
        "crop",
    ] {
        assert!(
            keys.iter().any(|k| k == key),
            "missing container param `{key}`"
        );
    }
}
