use super::*;

#[test]
fn every_chop_cooks_without_panicking() {
    let mut p = Patch::new();
    for spec in ops::all() {
        // Devices and sockets are exercised by their own tests; opening one
        // per operator here would fight for the same hardware.
        if spec.def.type_name.contains("audiodevice")
            || spec.def.type_name.contains("audiomovie")
            || spec.def.type_name.contains("osc")
            || spec.def.type_name.contains("midi")
            // A DMX Out broadcasts by default; leave the network alone.
            || spec.def.type_name.contains("dmx")
        {
            continue;
        }
        let id = p.add(spec.def.type_name, spec.def.type_name);
        p.run(id, 2);
        assert!(
            p.host.data(id).is_some(),
            "{} produced nothing",
            spec.def.type_name
        );
    }
}

#[test]
fn undoing_a_delete_can_cook_the_chop_again() {
    use otd_core::History;

    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "rows", Value::Int(1));
    p.run(c, 1);

    let mut history = History::default();
    history.checkpoint(&p.graph, "delete");
    p.host.engine.forget(c);
    p.cook.forget(c);
    p.graph.remove(c).unwrap();

    p.graph = history.undo(&p.graph).unwrap();
    p.host.engine.sync_with_graph(&p.graph);
    p.cook.sync_with_graph(&p.graph);

    p.run(c, 1);
    assert_eq!(p.value(c, "chan1"), 0.0);
}
