use otd_core::{Graph, OpRegistry, Value};
use otd_engine::{panel, panel_templates, registry};

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
fn button_internals_use_parameter_chop_and_executes() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let btn = add(&mut graph, &reg, root, panel::BUTTON, "button1");
    panel_templates::install_button(&mut graph, &reg, btn);

    let par1 = graph
        .children(btn)
        .iter()
        .find(|id| graph.node(**id).name == "par1")
        .copied()
        .unwrap();
    let text = graph
        .children(btn)
        .iter()
        .find(|id| graph.node(**id).name == "text")
        .copied()
        .unwrap();
    assert_eq!(graph.node(text).op_type, panel::TEXT);
    assert!(
        graph
            .children(text)
            .iter()
            .any(|id| graph.node(*id).op_type == "textTOP"),
        "textCOMP contains a textTOP internally, not at button level"
    );
    assert!(
        !graph
            .children(btn)
            .iter()
            .any(|id| graph.node(*id).op_type == "textTOP"),
        "button COMP must not expose a bare textTOP sibling"
    );

    assert_eq!(graph.node(par1).op_type, "parameterCHOP");
    assert_eq!(graph.node(par1).params["op"].value, Value::Str("..".into()));

    let parexec = graph
        .children(btn)
        .iter()
        .find(|id| graph.node(**id).name == "parexec1")
        .copied()
        .unwrap();
    assert_eq!(
        graph.node(parexec).params["op"].value,
        Value::Str("..".into())
    );
    assert_eq!(
        graph.node(parexec).params["parameters"].value,
        Value::Str("value".into())
    );
    assert_eq!(graph.node(text).params["text"].expression, "parent.label");
}

#[test]
fn legacy_button_text_top_is_migrated_to_text_comp() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let btn = add(&mut graph, &reg, root, panel::BUTTON, "button1");
    add(&mut graph, &reg, btn, "textTOP", "text1");
    panel_templates::ensure_button_interior(&mut graph, &reg, btn);
    assert!(
        !graph
            .children(btn)
            .iter()
            .any(|id| graph.node(*id).op_type == "textTOP"),
        "bare textTOP must not remain under button"
    );
    assert!(
        graph
            .children(btn)
            .iter()
            .any(|id| graph.node(*id).op_type == panel::TEXT),
        "button must have textCOMP child"
    );
}
