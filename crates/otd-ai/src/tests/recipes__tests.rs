use super::*;

    #[test]
    fn camera_examples_name_real_recipes_and_rotate() {
        for name in CAMERA_LOOKS {
            assert!(find(name).is_some(), "{name}");
        }
        let (a, b) = (camera_examples("", 0), camera_examples("", 1));
        assert!(a.contains("- blobhud:") && a.contains("PLAN:"));
        assert_ne!(a, b, "asking twice shows different worked examples");
        // Typed direction picks the examples.
        assert!(camera_examples("newspaper halftone", 0).contains(&find("halftone").unwrap().json));
    }

    #[test]
    fn every_file_parses_and_names_are_unique() {
        assert_eq!(all().len(), FILES.len(), "a recipe file failed to parse");
        let mut names: Vec<&str> = all().iter().map(|r| r.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), FILES.len());
    }

    #[test]
    fn the_header_is_stripped_from_what_the_model_sees() {
        for r in all() {
            let v: serde_json::Value = serde_json::from_str(&r.json).unwrap();
            for key in HEADER {
                assert!(
                    v.get(*key).is_none(),
                    "{}: `{key}` leaked into the plan",
                    r.name
                );
            }
            assert!(v.get("nodes").is_some(), "{}: no nodes", r.name);
        }
    }

    #[test]
    fn a_loop_request_is_shown_the_trails_recipe() {
        let text = examples_for("give it long ghostly trails", true);
        assert!(text.contains("REQUEST: give the video long smeary trails"));
        assert_eq!(text.matches("REQUEST:").count(), 2);
    }

    #[test]
    fn a_3d_request_is_shown_3d() {
        let text = examples_for("a spinning torus rendered in 3d", false);
        assert!(text.contains("torusSOP"), "{text}");
    }

    #[test]
    fn with_source_rewrites_every_mention() {
        let mut plan = Plan::default();
        plan.connections.push(patch::PlannedWire {
            from: SOURCE.into(),
            to: "x".into(),
            input: 0,
        });
        let mut params = BTreeMap::new();
        params.insert("target".to_string(), Value::Str(SOURCE.into()));
        let mut exports = BTreeMap::new();
        exports.insert("scale".to_string(), format!("{SOURCE}:r"));
        plan.nodes.push(PlannedNode {
            name: "x".into(),
            op: "feedbackTOP".into(),
            pos: [0.0, 0.0],
            params,
            expressions: BTreeMap::new(),
            exports,
        });
        with_source(&mut plan, "movie1");
        assert_eq!(plan.connections[0].from, "movie1");
        assert_eq!(plan.nodes[0].params["target"], Value::Str("movie1".into()));
        assert_eq!(plan.nodes[0].exports["scale"], "movie1:r");
    }
