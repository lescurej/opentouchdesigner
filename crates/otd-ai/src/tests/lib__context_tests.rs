use super::*;

    #[test]
    fn conversation_is_bounded_unicode_safe_and_clearable() {
        let mut history = Conversation::default();
        for i in 0..9 {
            history.record(&format!("request {i}"), &"🎨".repeat(3000));
        }
        assert_eq!(history.turns.len(), 6);
        assert_eq!(history.turns[0].0, "request 3");
        assert_eq!(history.turns[0].1.chars().count(), 2000);
        history.clear();
        assert!(history.context().is_empty());
    }

    #[test]
    fn request_carries_recipe_history_and_current_network() {
        let registry = otd_engine::registry();
        let graph = Graph::new();
        let mut history = Conversation::default();
        history.record("Applied recipe smoke", "Blue ink with slow motion");
        let ask = Ask {
            provider: Provider::Anthropic,
            model: String::new(),
            prompt: "make it slower".into(),
            image: None,
            seen: Default::default(),
            graph: &graph,
            parent: graph.root(),
            selected: None,
            viewer: None,
            registry: &registry,
            allow_delete: false,
            scope: &[],
        };
        let request = request_with_history(&ask, &history);
        assert!(request.user.contains("Applied recipe smoke"));
        assert!(request.user.contains("make it slower"));
        assert!(request.user.contains(&patch::describe(
            &graph,
            graph.root(),
            None,
            None,
            &[],
            &registry
        )));
    }

    #[test]
    fn each_kind_of_picture_gets_its_own_brief() {
        let registry = otd_engine::registry();
        let graph = Graph::new();
        let frame = vision::Image::from_rgba(4, 2, vec![128; 32]).unwrap();
        let user = |seen: Seen| {
            request_for(&Ask {
                provider: Provider::Anthropic,
                model: String::new(),
                prompt: String::new(),
                image: Some(frame.clone()),
                seen,
                graph: &graph,
                parent: graph.root(),
                selected: None,
                viewer: None,
                registry: &registry,
                allow_delete: false,
                scope: &[],
            })
            .user
        };
        assert!(user(Seen::Reference).contains("REFERENCE IMAGE"));
        let camera = user(Seen::Camera);
        assert!(camera.contains("LIVE CAMERA") && !camera.contains("REFERENCE IMAGE"));
        let result = user(Seen::Result(vec!["BLACK", "PASSTHROUGH"]));
        assert!(result.contains("- BLACK:") && result.contains("- PASSTHROUGH:"));
        assert!(!result.contains("- BLOWN:"), "only the measured faults are explained");
    }
