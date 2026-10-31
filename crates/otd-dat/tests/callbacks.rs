use otd_core::{EvalContext, Graph, Value};
use otd_dat::callbacks::{
    default_callbacks_dat_pos, install_callbacks_dat, resolve_callback_script,
};

#[test]
fn external_callbacks_dat_overrides_inline_source() {
    let reg = otd_dat::ops::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let ex_def = reg.get("executeDAT").unwrap().clone();
    let text_def = reg.get("textDAT").unwrap().clone();
    let ex = graph.create(root, &ex_def, Some("exec1")).unwrap();
    let cb = graph.create(root, &text_def, Some("exec1_callbacks")).unwrap();
    graph
        .set_param(
            cb,
            "text",
            Value::Str("def onFrameStart(frame):\n    pass\n".into()),
        )
        .unwrap();
    graph
        .set_param(ex, "callbacks", Value::Str("./exec1_callbacks".into()))
        .unwrap();
    graph
        .set_param(
            ex,
            "source",
            Value::Str("def onFrameStart(frame):\n    raise RuntimeError('inline')\n".into()),
        )
        .unwrap();

    let eval = EvalContext::default();
    let script = resolve_callback_script(&graph, ex, &eval, "callbacks", "source");
    assert!(script.contains("pass"));
    assert!(!script.contains("inline"));
}

#[test]
fn install_creates_sibling_and_wires_param() {
    let reg = otd_dat::ops::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let def = reg.get("webclientDAT").unwrap().clone();
    let id = graph.create(root, &def, Some("webclient1")).unwrap();
    let cb = install_callbacks_dat(&mut graph, &reg, id).expect("installed");
    assert_eq!(graph.node(cb).name, "webclient1_callbacks");
    let path = graph.node(id).params["callbacks"].value.as_str();
    assert_eq!(path, "./webclient1_callbacks");
    assert_eq!(graph.node(cb).callback_owner, Some(id));
    assert!(graph.node(id).params["source"].value.as_str().trim().is_empty());
    let owner_pos = graph.node(id).pos;
    assert_eq!(graph.node(cb).pos, default_callbacks_dat_pos(owner_pos));
}

#[test]
fn link_updates_callback_owner_field() {
    let reg = otd_dat::ops::registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let ex_def = reg.get("executeDAT").unwrap().clone();
    let text_def = reg.get("textDAT").unwrap().clone();
    let ex = graph.create(root, &ex_def, Some("exec1")).unwrap();
    let cb = graph.create(root, &text_def, Some("exec1_callbacks")).unwrap();
    otd_dat::callbacks::link_callbacks_dat(&mut graph, ex, cb);
    assert_eq!(graph.node(cb).callback_owner, Some(ex));
}
