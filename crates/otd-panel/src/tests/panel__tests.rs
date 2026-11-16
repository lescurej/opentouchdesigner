use super::*;

use otd_core::OpRegistry;
    use otd_core::Value;

    fn install_registry() -> OpRegistry {
        let mut reg = OpRegistry::new();
        register(&mut reg);
        otd_chop::register_all(&mut reg);
        otd_dat::register_all(&mut reg);
        otd_gpu::register_all(&mut reg);
        reg
    }

    fn graph_with(op: &str, name: &str) -> (Graph, NodeId) {
        let mut reg = OpRegistry::new();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        let id = graph
            .create(root, reg.get(op).unwrap(), Some(name))
            .unwrap();
        (graph, id)
    }

    #[test]
    fn a_widget_reads_its_layout_and_state_from_parameters() {
        let (mut graph, id) = graph_with(SLIDER, "fader1");
        graph.set_param(id, "x", Value::Float(0.25)).unwrap();
        graph.set_param(id, "value", Value::Float(0.7)).unwrap();
        graph.set_param(id, "max", Value::Float(4.0)).unwrap();

        let w = widgets(&graph);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].kind, Kind::Slider);
        assert_eq!(w[0].rect[0], 0.25);
        assert_eq!(w[0].value, 0.7);
        assert_eq!(w[0].range, (0.0, 4.0));
    }

    #[test]
    fn an_unlabelled_widget_shows_its_node_name() {
        let (graph, _) = graph_with(BUTTON, "go");
        assert_eq!(widgets(&graph)[0].label, "go");
    }

    #[test]
    fn interior_text_comp_is_not_a_separate_widget() {
        let reg = install_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let btn = graph
            .create(root, reg.get(BUTTON).unwrap(), Some("button1"))
            .unwrap();
        crate::templates::install_button(&mut graph, &reg, btn);
        let w = widgets(&graph);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].id, btn);
    }

    #[test]
    fn widgets_keep_a_stable_order_so_they_stay_clickable() {
        let mut reg = OpRegistry::new();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        // Created out of order on purpose.
        for name in ["zed", "alpha", "mid"] {
            graph
                .create(root, reg.get(BUTTON).unwrap(), Some(name))
                .unwrap();
        }
        let names: Vec<String> = widgets(&graph).iter().map(|w| w.label.clone()).collect();
        assert_eq!(names, vec!["alpha", "mid", "zed"]);
    }

    #[test]
    fn container_view_defaults_to_400_by_300() {
        let mut reg = OpRegistry::new();
        register(&mut reg);
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
        let mut reg = OpRegistry::new();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        let outer = graph
            .create(root, reg.get(CONTAINER).unwrap(), Some("outer"))
            .unwrap();
        let inner = graph
            .create(outer, reg.get(CONTAINER).unwrap(), Some("inner"))
            .unwrap();
        let btn = graph
            .create(inner, reg.get(BUTTON).unwrap(), Some("go"))
            .unwrap();
        let outer_w = panel_display_widgets(&graph, outer);
        assert_eq!(outer_w.len(), 1);
        assert_eq!(outer_w[0].id, inner);
        assert!(outer_w.iter().all(|w| w.id != btn));
        let inner_w = panel_display_widgets(&graph, inner);
        assert_eq!(inner_w.len(), 1);
        assert_eq!(inner_w[0].id, btn);
    }

    #[test]
    fn nothing_but_a_panel_comp_is_a_widget() {
        let mut reg = otd_gpu::ops::registry();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        graph
            .create(root, reg.get("noiseTOP").unwrap(), Some("noise1"))
            .unwrap();
        graph
            .create(root, reg.get(BUTTON).unwrap(), Some("go"))
            .unwrap();
        assert_eq!(widgets(&graph).len(), 1);
    }
