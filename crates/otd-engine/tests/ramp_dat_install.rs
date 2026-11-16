use otd_core::{Graph, Value};
use otd_engine::registry;
use otd_gpu::ramp_dat::{DEFAULT_RAMP_TABLE, PARAM_DAT, install_ramp_dat, resolved_ramp_dat};

#[test]
fn new_ramp_gets_a_linked_keys_table() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let ramp = graph
        .create(root, reg.get("rampTOP").unwrap(), Some("ramp1"))
        .unwrap();
    let dat = install_ramp_dat(&mut graph, &reg, ramp).expect("installed");
    assert_eq!(graph.node(dat).op_type, "tableDAT");
    assert_eq!(
        graph.node(ramp).param(PARAM_DAT).unwrap().value.as_str(),
        "./ramp1_keys"
    );
    assert_eq!(resolved_ramp_dat(&graph, ramp), Some(dat));
    let text = graph.node(dat).param("text").unwrap().value.as_str();
    assert_eq!(text, DEFAULT_RAMP_TABLE);
}

#[test]
fn install_is_skipped_when_dat_is_already_set() {
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let ramp = graph
        .create(root, reg.get("rampTOP").unwrap(), Some("ramp1"))
        .unwrap();
    graph
        .set_param(ramp, PARAM_DAT, Value::Str("/custom".into()))
        .unwrap();
    assert!(install_ramp_dat(&mut graph, &reg, ramp).is_none());
}
