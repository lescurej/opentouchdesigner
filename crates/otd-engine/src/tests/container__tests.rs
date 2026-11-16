use super::*;

    #[test]
    fn container_view_defaults_to_400_by_300() {
        let reg = crate::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let id = graph
            .create(root, reg.get(CONTAINER).unwrap(), Some("ui"))
            .unwrap();
        let (w, h) = container_view_pixel_size(&graph, id);
        assert_eq!(w, 400.0);
        assert_eq!(h, 300.0);
    }

    #[test]
    fn container_display_is_direct_children_only() {
        let reg = crate::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let outer = graph
            .create(root, reg.get(CONTAINER).unwrap(), Some("outer"))
            .unwrap();
        let inner = graph
            .create(outer, reg.get(CONTAINER).unwrap(), Some("inner"))
            .unwrap();
        let leaf = graph
            .create(inner, reg.get(crate::comp::BASE).unwrap(), Some("widget"))
            .unwrap();
        let outer_w = panel_display_widgets(&graph, outer);
        assert_eq!(outer_w.len(), 1);
        assert_eq!(outer_w[0].id, inner);
        assert!(outer_w.iter().all(|w| w.id != leaf));
        let inner_w = panel_display_widgets(&graph, inner);
        assert_eq!(inner_w.len(), 1);
        assert_eq!(inner_w[0].id, leaf);
    }

    #[test]
    fn panel_child_base_comp_under_container_including_nested_clones() {
        let reg = crate::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let c = graph
            .create(root, reg.get(CONTAINER).unwrap(), Some("ui"))
            .unwrap();
        let host = graph
            .create(c, reg.get(crate::comp::BASE).unwrap(), Some("host"))
            .unwrap();
        let template = graph
            .create(host, reg.get(crate::comp::BASE).unwrap(), Some("template"))
            .unwrap();
        let instance = graph
            .create(host, reg.get(crate::comp::BASE).unwrap(), Some("instance"))
            .unwrap();
        graph.set_clone(instance, Some("/ui/host/template"));
        assert!(is_panel_child_base_comp(&graph, host));
        assert!(is_panel_child_base_comp(&graph, template));
        assert!(is_panel_child_base_comp(&graph, instance));
        assert!(!is_panel_child_base_comp(&graph, c));
        assert!(!is_panel_child_base_comp(&graph, root));
    }

    #[test]
    fn panel_child_not_outside_container_tree() {
        let reg = crate::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let parent = graph
            .create(root, reg.get(crate::comp::BASE).unwrap(), Some("parent"))
            .unwrap();
        let instance = graph
            .create(
                parent,
                reg.get(crate::comp::BASE).unwrap(),
                Some("instance"),
            )
            .unwrap();
        graph.set_clone(instance, Some("/parent/template"));
        assert!(!is_panel_child_base_comp(&graph, parent));
        assert!(!is_panel_child_base_comp(&graph, instance));
    }

    #[test]
    fn child_layout_uses_default_fractions_without_params() {
        let reg = crate::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let c = graph
            .create(root, reg.get(CONTAINER).unwrap(), Some("ui"))
            .unwrap();
        let kid = graph
            .create(c, reg.get(crate::comp::BASE).unwrap(), Some("a"))
            .unwrap();
        let w = widgets(&graph);
        let child = w.iter().find(|w| w.id == kid).unwrap();
        assert!((child.rect[0] - 0.05).abs() < 1e-4);
    }
