use super::*;

use super::*;
    use crate::test_ops::{TEST_PASS, registry};
    use crate::value::Value;

    fn sample() -> (Graph, OpRegistry) {
        let reg = registry();
        let mut g = Graph::new();
        let root = g.root();
        let comp = g.create(root, reg.get("container").unwrap(), None).unwrap();
        let a = g.create(comp, reg.get("pass").unwrap(), None).unwrap();
        let b = g.create(root, reg.get("comp2").unwrap(), None).unwrap();
        let m = g.create(root, reg.get("movie").unwrap(), None).unwrap();
        g.connect(m, b, 0).unwrap();
        g.set_param(a, TEST_PASS, Value::Float(0.375)).unwrap();
        g.set_expression(b, TEST_PASS, "sin(absTime) * 0.5")
            .unwrap();
        g.node_mut(a).pos = [120.0, -40.0];
        g.node_mut(b).flags.bypass = true;
        (g, reg)
    }

    #[test]
    fn round_trips_through_text() {
        let (g, reg) = sample();
        let text = Project::from_graph(&g, &reg, 60.0).to_ron().unwrap();
        let g2 = Project::from_ron(&text).unwrap().to_graph(&reg).unwrap();

        assert_eq!(g.len(), g2.len());
        for id in g.walk() {
            let path = g.path(id);
            let id2 = g2.find(&path).unwrap_or_else(|| panic!("missing {path}"));
            let (n, n2) = (g.node(id), g2.node(id2));
            assert_eq!(n.op_type, n2.op_type);
            assert_eq!(n.pos, n2.pos);
            assert_eq!(n.flags, n2.flags);
            for (k, p) in &n.params {
                let p2 = &n2.params[k];
                assert_eq!(p.mode, p2.mode, "{path}.{k} mode");
                assert_eq!(p.value, p2.value, "{path}.{k} value");
                assert_eq!(p.expression, p2.expression, "{path}.{k} expression");
            }
            for (i, slot) in n.inputs.iter().enumerate() {
                assert_eq!(
                    slot.map(|s| g.path(s)),
                    n2.inputs[i].map(|s| g2.path(s)),
                    "{path} input {i}"
                );
            }
        }

        // Second round trip must be byte-identical: no churn in git.
        let text2 = Project::from_graph(&g2, &reg, 60.0).to_ron().unwrap();
        assert_eq!(text, text2);
    }

    #[test]
    fn expressions_are_live_again_after_loading() {
        let (g, reg) = sample();
        let text = Project::from_graph(&g, &reg, 60.0).to_ron().unwrap();
        let g2 = Project::from_ron(&text).unwrap().to_graph(&reg).unwrap();
        let b = g2.find("/comp1").unwrap();
        assert!(
            g2.node(b).has_time_dependent_param(),
            "a loaded expression must re-arm time dependence"
        );
    }

    #[test]
    fn untouched_parameters_are_not_written() {
        let (g, reg) = sample();
        let text = Project::from_graph(&g, &reg, 60.0).to_ron().unwrap();
        let movie_block = text
            .lines()
            .skip_while(|l| !l.contains("path: \"/movie1\""))
            .take(4)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !movie_block.contains("params"),
            "default params leaked into the file:\n{text}"
        );
    }

    #[test]
    fn an_unknown_operator_becomes_a_placeholder_not_a_load_failure() {
        let reg = registry();
        let project = Project {
            format: FORMAT_ID.to_string(),
            version: FORMAT_VERSION,
            fps: 60.0,
            sample_rate: PROJECT_SAMPLE_RATE_DEFAULT,
            nodes: vec![NodeEntry {
                path: "/resample1".into(),
                op: "resampleCHOP".into(),
                pos: (0.0, 0.0),
                size: None,
                inputs: Vec::new(),
                params: IndexMap::new(),
                flags: NodeFlags::default(),
                external: None,
                clone: None,
                callback_owner: None,
                docked_to: None,
                dock_shown: true,
            }],
            settings: ProjectSettings::default(),
            editor: ProjectEditor::default(),
        };
        let graph = project.to_graph(&reg).unwrap();
        let id = graph.find("/resample1").unwrap();
        assert_eq!(graph.node(id).op_type, crate::missing::MISSING_CHOP);
        assert_eq!(
            graph
                .node(id)
                .param(crate::missing::ORIGINAL_TYPE_PARAM)
                .map(|p| p.value.as_str().to_string()),
            Some("resampleCHOP".to_string())
        );
        assert_eq!(graph.load_warnings.len(), 1);
    }

    #[test]
    fn the_default_sample_rate_is_not_written() {
        let (g, reg) = sample();
        let text = Project::from_graph(&g, &reg, 60.0).to_ron().unwrap();
        assert!(
            !text.contains("sample_rate"),
            "48 kHz leaked into the file:\n{text}"
        );
    }

    #[test]
    fn a_chosen_sample_rate_round_trips() {
        let (g, reg) = sample();
        let mut p = Project::from_graph(&g, &reg, 60.0);
        p.sample_rate = 96_000.0;
        let back = Project::from_ron(&p.to_ron().unwrap()).unwrap();
        assert!((back.sample_rate - 96_000.0).abs() < 1.0);
        let legacy =
            Project::from_ron(&Project::from_graph(&g, &reg, 24.0).to_ron().unwrap()).unwrap();
        assert!((legacy.sample_rate - 48_000.0).abs() < 1.0);
    }

    #[test]
    fn a_file_without_an_editor_block_still_opens() {
        let project = Project::from_ron(
            r#"(
  format: "opentouchdesigner-project",
  version: 1,
  fps: 60.0,
  nodes: [],
)
"#,
        )
        .unwrap();
        assert!(project.editor.network.is_empty());
        assert!(project.editor.views.is_empty());
    }

    #[test]
    fn the_network_view_round_trips_and_stays_out_of_an_untouched_file() {
        let (g, reg) = sample();
        let plain = Project::from_graph(&g, &reg, 60.0).to_ron().unwrap();
        assert!(
            !plain.contains("editor"),
            "a default camera leaked into the file:\n{plain}"
        );

        let mut project = Project::from_graph(&g, &reg, 60.0);
        project.editor.network = "/comp1".into();
        project.editor.views.insert(
            "/".into(),
            NetworkView {
                pan: (12.0, -3.5),
                zoom: 1.0,
            },
        );
        project.editor.views.insert(
            "/comp1".into(),
            NetworkView {
                pan: (0.0, 0.0),
                zoom: 0.5,
            },
        );
        let text = project.to_ron().unwrap();
        let back = Project::from_ron(&text).unwrap();
        assert_eq!(back.editor, project.editor);
        let root = text
            .split("\"/\":")
            .nth(1)
            .unwrap()
            .split("\"/comp1\"")
            .next()
            .unwrap();
        assert!(!root.contains("zoom"), "zoom 1 should be omitted:\n{root}");
        assert!(text.contains("network: \"/comp1\""));
    }

    #[test]
    fn a_future_format_version_is_refused() {
        let (g, reg) = sample();
        let mut p = Project::from_graph(&g, &reg, 60.0);
        p.version = FORMAT_VERSION + 1;
        assert!(matches!(
            p.to_graph(&reg).unwrap_err(),
            ProjectError::TooNew(_)
        ));
    }
