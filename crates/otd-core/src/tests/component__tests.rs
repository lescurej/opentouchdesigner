use super::*;
    use crate::test_ops::registry as test_registry;
    use crate::value::Value;

    fn build(graph: &mut Graph, reg: &OpRegistry, parent: NodeId, op: &str, name: &str) -> NodeId {
        let def = reg.get(op).unwrap().clone();
        graph.create(parent, &def, Some(name)).unwrap()
    }

    fn master(graph: &mut Graph, reg: &OpRegistry, name: &str) -> NodeId {
        let root = graph.root();
        let comp = build(graph, reg, root, "container", name);
        graph.add_custom_param(comp, "gain", Param::float(1.0));
        let a = build(graph, reg, comp, "pass", "a");
        let b = build(graph, reg, comp, "pass", "b");
        graph.connect(a, b, 0).unwrap();
        comp
    }

    #[test]
    fn a_component_round_trips_through_its_own_file() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let comp = master(&mut graph, &reg, "rig");
        graph.set_param(comp, "gain", Value::Float(2.0)).unwrap();

        let saved = Component::from_graph(&graph, comp, &reg).unwrap();
        let text = saved.to_ron().unwrap();
        // Paths inside the file are relative, so the same component can be
        // dropped in anywhere.
        assert!(text.contains("\"/a\""), "{text}");
        assert!(!text.contains("/rig/"), "{text}");

        let reloaded = Component::from_ron(&text).unwrap();
        let mut other = Graph::new();
        let root = other.root();
        let target = build(&mut other, &reg, root, "container", "elsewhere");
        reloaded.expand_into(&mut other, target, &reg).unwrap();

        assert!(other.find("/elsewhere/a").is_some());
        assert_eq!(
            other.node(other.find("/elsewhere/b").unwrap()).inputs[0],
            other.find("/elsewhere/a")
        );
        assert_eq!(
            other.node(target).param("gain").unwrap().value,
            Value::Float(2.0)
        );
    }

    #[test]
    fn re_expanding_keeps_the_instance_settings() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let comp = master(&mut graph, &reg, "rig");
        let saved = Component::from_graph(&graph, comp, &reg).unwrap();

        // The artist dials in a value on this instance...
        graph.set_param(comp, "gain", Value::Float(3.5)).unwrap();
        // ...and the shared definition is re-read.
        saved.expand_into(&mut graph, comp, &reg).unwrap();
        assert_eq!(
            graph.node(comp).param("gain").unwrap().value,
            Value::Float(3.5),
            "re-expanding must not reset the instance"
        );
    }

    #[test]
    fn renaming_a_master_updates_clone_of_references() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let m = master(&mut graph, &reg, "master");
        let root = graph.root();
        let c = build(&mut graph, &reg, root, "container", "copy");
        graph.set_clone(c, Some("/master"));
        assert_eq!(graph.sync_clones(&reg), 1);

        let old_path = graph.path(m);
        graph.node_mut_quiet(m).name = "leader".into();
        graph.refresh_clone_paths_after_rename(&old_path, &graph.path(m));
        assert_eq!(
            graph.node(c).clone_of.as_deref(),
            Some("/leader"),
            "clone_of must track the master's new path"
        );

        build(&mut graph, &reg, m, "pass", "extra");
        assert_eq!(
            graph.sync_clones(&reg),
            1,
            "clones must still follow after the master is renamed"
        );
        assert!(graph.find("/copy/extra").is_some());
    }

    #[test]
    fn a_clone_follows_its_master_but_keeps_its_own_settings() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let m = master(&mut graph, &reg, "master");
        let root = graph.root();
        let c = build(&mut graph, &reg, root, "container", "copy");
        graph.set_clone(c, Some("/master"));

        assert_eq!(graph.sync_clones(&reg), 1);
        assert!(graph.find("/copy/a").is_some());
        assert!(graph.find("/copy/b").is_some());

        // Its own setting, then a change to the master's structure.
        graph.set_param(c, "gain", Value::Float(9.0)).unwrap();
        assert_eq!(graph.sync_clones(&reg), 0, "an unchanged master is free");

        graph.set_param(m, "gain", Value::Float(0.25)).unwrap();
        assert_eq!(
            graph.sync_clones(&reg),
            1,
            "master edits bump subtree revision"
        );
        assert_eq!(
            graph.node(c).param("gain").unwrap().value,
            Value::Float(9.0),
            "master gain must not overwrite the clone"
        );

        build(&mut graph, &reg, m, "pass", "c");
        graph.add_custom_param(m, "mix", Param::float(0.5));
        assert_eq!(graph.sync_clones(&reg), 1);
        assert!(graph.find("/copy/c").is_some(), "the clone followed");
        assert_eq!(
            graph.node(c).param("gain").unwrap().value,
            Value::Float(9.0),
            "the clone kept its own value"
        );
        assert!(
            graph.node(c).param("mix").is_none(),
            "new master custom params must not appear on the clone"
        );
    }

    #[test]
    fn a_clone_of_an_ancestor_is_refused_rather_than_expanding_forever() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let outer = master(&mut graph, &reg, "outer");
        let inner = build(&mut graph, &reg, outer, "container", "inner");
        graph.set_clone(inner, Some("/outer"));
        assert_eq!(graph.sync_clones(&reg), 0);
        assert!(graph.find("/outer/inner/inner").is_none());
    }

    #[test]
    fn set_clone_noop_keeps_sync_state_and_internal_wires() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let _master = master(&mut graph, &reg, "master");
        let root = graph.root();
        let c = build(&mut graph, &reg, root, "container", "copy");
        graph.set_clone(c, Some("/master"));
        assert_eq!(graph.sync_clones(&reg), 1);
        let a = graph.find("/copy/a").unwrap();
        let b = graph.find("/copy/b").unwrap();
        assert!(graph.node(b).inputs[0] == Some(a));

        let synced_at = graph.node(c).clone_synced_at;
        graph.set_clone(c, Some("/master"));
        assert_eq!(graph.node(c).clone_synced_at, synced_at);
        assert_eq!(graph.sync_clones(&reg), 0);
        assert_eq!(graph.node(b).inputs[0], Some(a));
    }

    #[test]
    fn a_dangling_clone_reference_is_ignored() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let c = build(&mut graph, &reg, root, "container", "copy");
        graph.set_clone(c, Some("/nowhere"));
        assert_eq!(graph.sync_clones(&reg), 0);
    }
