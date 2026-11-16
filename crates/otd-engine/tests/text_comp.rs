use otd_core::Graph;
use otd_engine::{panel_templates, registry, text_comp};

#[test]
fn text_comp_registers_and_gets_interior_on_install() {
    let reg = registry();
    assert!(reg.get(text_comp::TEXT).is_some());

    let mut graph = Graph::new();
    let root = graph.root();
    let id = graph
        .create(root, reg.get(text_comp::TEXT).unwrap(), Some("label1"))
        .unwrap();
    panel_templates::install_text_comp_interior(&mut graph, &reg, id);

    let text_top = graph
        .children(id)
        .iter()
        .find(|&&cid| graph.node(cid).op_type == "textTOP")
        .copied()
        .expect("textTOP");
    let out = graph
        .children(id)
        .iter()
        .find(|&&cid| graph.node(cid).name == "out1")
        .copied()
        .expect("out1");
    assert_eq!(graph.resolve_input_source(out, 0), Some(text_top));
    assert_eq!(
        graph.node(text_top).params["text"].expression.as_str(),
        "parent.text"
    );
}

#[test]
fn text_comp_default_layout_fractions() {
    let reg = registry();
    let mut graph = Graph::new();
    let id = graph
        .create(graph.root(), reg.get(text_comp::TEXT).unwrap(), Some("t"))
        .unwrap();
    let w = graph.node(id).params["w"].value.as_f64();
    assert!((w - 0.2).abs() < 1e-6, "default width fraction {w}");
}
