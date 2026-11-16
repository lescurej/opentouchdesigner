use super::*;
    use crate::Graph;

    #[test]
    fn split_relative_accepts_single_name() {
        assert_eq!(split_relative("in1"), Some(("".into(), "in1".into())));
        assert_eq!(
            split_relative("geo/in1"),
            Some(("geo".into(), "in1".into()))
        );
    }

    #[test]
    fn copy_paste_preserves_clone_reference_on_comp() {
        use crate::component::Component;

        let reg = crate::test_ops::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let master = graph
            .create(root, reg.get("container").unwrap(), Some("master"))
            .unwrap();
        graph
            .create(master, reg.get("pass").unwrap(), Some("a"))
            .unwrap();
        let saved = Component::from_graph(&graph, master, &reg).unwrap();
        let clone = graph
            .create(root, reg.get("container").unwrap(), Some("copy"))
            .unwrap();
        graph.set_clone(clone, Some("/master"));
        saved.expand_into(&mut graph, clone, &reg).unwrap();

        let clip = copy_selection(&graph, root, &[clone], &reg);
        let top = clip
            .nodes
            .iter()
            .find(|e| e.path == "copy")
            .expect("cloned comp in clipboard");
        assert_eq!(top.clone.as_deref(), Some("/master"));

        let pasted = paste(&mut graph, root, &reg, &clip, [300.0, 80.0]).unwrap();
        let pasted_clone = pasted
            .iter()
            .find(|&&id| graph.path(id) == "/copy1")
            .copied()
            .expect("pasted clone comp");
        assert_eq!(
            graph.node(pasted_clone).clone_of.as_deref(),
            Some("/master")
        );
    }

    #[test]
    fn copy_paste_keeps_internal_wires() {
        let reg = crate::test_ops::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let a = graph
            .create(root, reg.get("pass").unwrap(), Some("a"))
            .unwrap();
        let b = graph
            .create(root, reg.get("pass").unwrap(), Some("b"))
            .unwrap();
        graph.connect(a, b, 0).unwrap();

        let clip = copy_selection(&graph, root, &[a, b], &reg);
        assert_eq!(clip.nodes.len(), 2);

        let pasted = paste(&mut graph, root, &reg, &clip, [400.0, 100.0]).unwrap();
        assert_eq!(pasted.len(), 2);
        let wired = pasted
            .iter()
            .find(|id| graph.node(**id).inputs[0].is_some())
            .copied()
            .unwrap();
        assert!(graph.node(wired).inputs[0].is_some());
    }
