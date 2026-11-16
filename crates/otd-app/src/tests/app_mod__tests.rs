use super::*;
use otd_core::Project;

    /// Noise -> level -> out, plus an unrelated branch nobody wired up.
    fn patch() -> (Graph, OpRegistry, NodeId, NodeId) {
        let registry = otd_engine::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let noise = graph
            .create(root, registry.get("noiseTOP").unwrap(), Some("noise1"))
            .unwrap();
        let out = graph
            .create(root, registry.get("nullTOP").unwrap(), Some("out1"))
            .unwrap();
        graph.connect(noise, out, 0).unwrap();
        let spare = graph
            .create(root, registry.get("rampTOP").unwrap(), Some("ramp1"))
            .unwrap();
        (graph, registry, out, spare)
    }

    #[test]
    fn a_node_on_screen_cooks_and_the_same_node_off_screen_does_not() {
        let (graph, _reg, out, spare) = patch();

        // On the canvas: its own viewer makes it a root.
        let root = graph.root();
        let roots = ui::cook_roots::cook_roots(&graph, Some(out), &[spare], root);
        assert!(roots.contains(&spare));

        // Panned away: nothing wants it, so nothing cooks it.
        let roots = ui::cook_roots::cook_roots(&graph, Some(out), &[], root);
        assert!(!roots.contains(&spare));
        assert!(roots.contains(&out), "perform output chain stays a root");
    }

    #[test]
    fn perform_mode_leaves_only_the_output_chain() {
        let (mut graph, _reg, out, spare) = patch();
        // A render-flagged node is a root wherever the canvas is looking —
        // that is what the flag means, and a show output relies on it.
        graph.node_mut(spare).flags.render = true;

        // Perform mode is exactly "nothing is on the canvas".
        let performing = ui::cook_roots::cook_roots(&graph, Some(out), &[], graph.root());
        assert!(performing.contains(&out));
        assert!(
            performing.contains(&spare),
            "the render flag survives perform mode"
        );

        let noise = graph.find("/noise1").unwrap();
        assert!(
            !performing.contains(&noise),
            "an upstream node is pulled by its consumer, not listed as a root"
        );
    }

    #[test]
    fn a_deleted_viewer_does_not_become_a_root() {
        let (mut graph, _reg, out, _) = patch();
        graph.remove(out).unwrap();
        // A stale NodeId from a deleted node must not reach the cook.
        assert!(ui::cook_roots::cook_roots(&graph, Some(out), &[], graph.root()).is_empty());
    }

    #[test]
    fn cook_roots_tolerates_stale_current_network_id() {
        let graph = Graph::new();
        let roots = ui::cook_roots::cook_roots(&graph, None, &[], NodeId::default());
        assert!(roots.is_empty());
    }

    #[test]
    fn network_pan_and_zoom_round_trip_for_root_and_comp() {
        let registry = otd_engine::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let comp = graph
            .create(root, registry.get("baseCOMP").unwrap(), Some("comp1"))
            .unwrap();
        let untouched = capture_editor_camera(&graph, root, CanvasView::default(), &HashMap::new());
        assert!(untouched.network.is_empty());
        assert!(untouched.views.is_empty());

        let mut stored = HashMap::new();
        stored.insert(
            root,
            CanvasView {
                pan: egui::vec2(40.0, -8.0),
                zoom: 1.25,
            },
        );
        let inside = CanvasView {
            pan: egui::vec2(12.0, 30.0),
            zoom: 0.5,
        };
        let mut project = Project::from_graph(&graph, &registry, 60.0);
        project.editor = capture_editor_camera(&graph, comp, inside, &stored);
        let loaded = Project::from_ron(&project.to_ron().unwrap()).unwrap();
        let back = loaded.to_graph(&registry).unwrap();
        let (current, view, views) = restore_editor_camera(&back, &loaded.editor);
        assert_eq!(back.path(current), "/comp1");
        assert!((view.zoom - 0.5).abs() < 1e-4);
        assert!((view.pan.x - 12.0).abs() < 1e-4);
        assert!((view.pan.y - 30.0).abs() < 1e-4);
        let root_view = views.get(&back.root()).copied().unwrap();
        assert!((root_view.zoom - 1.25).abs() < 1e-4);
        assert!((root_view.pan.x - 40.0).abs() < 1e-4);
        assert!((root_view.pan.y - -8.0).abs() < 1e-4);
    }
