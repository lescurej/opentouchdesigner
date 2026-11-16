use otd_core::{Graph, NodeId, OpRegistry, Value};

use super::describe::clip;
use super::helpers::{json_to_value, split_export};
use super::*;

    use otd_core::indexmap::IndexMap;
    use otd_core::{Connector, Family, OpDef, Param};

    /// A two-operator registry of this crate's own. The real one lives in
    /// `otd-engine`, which depends on the GPU; validation logic should not.
    fn test_registry() -> OpRegistry {
        fn pass_params() -> IndexMap<String, Param> {
            let mut m = IndexMap::new();
            m.insert("gain".into(), Param::float(1.0).with_label("Gain"));
            m.insert(
                "mode".into(),
                Param::menu("over", &["over", "add"]).with_label("Mode"),
            );
            m.insert("size".into(), Param::new(Value::Vec2([1.0, 1.0])));
            // A path reference, the marker `apply` rewrites bare names for.
            m.insert(
                "camera".into(),
                Param::str("").with_label("Camera").as_path_ref(),
            );
            m
        }
        let mut r = OpRegistry::new();
        r.register(OpDef {
            type_name: "pass",
            input_families: &[],
            label: "Pass",
            family: Family::Top,
            inputs: &["in"],
            summary: "",
            time_dependent: false,
            params: pass_params,
            connector: Connector::None,
        });
        // A CHOP, so exports have something legitimate to read from — and so
        // the "that is a TOP" refusal has something to contrast with.
        fn beat_params() -> IndexMap<String, Param> {
            let mut m = IndexMap::new();
            m.insert("rate".into(), Param::float(1.0).with_label("Rate"));
            m
        }
        r.register(OpDef {
            type_name: "beat",
            input_families: &[],
            label: "Beat",
            family: Family::Chop,
            inputs: &[],
            summary: "",
            time_dependent: true,
            params: beat_params,
            connector: Connector::None,
        });
        r
    }

    fn plan_json() -> serde_json::Value {
        serde_json::json!({
            "notes": "a chain",
            "nodes": [
                { "name": "a", "op": "pass", "pos": [0, 0] },
                { "name": "b", "op": "pass", "pos": [200, 0] }
            ],
            "connections": [{ "from": "a", "to": "b", "input": 0 }],
            "viewer": "b"
        })
    }

    #[test]
    fn json_survives_prose_and_fences_around_it() {
        let bare = r#"{"nodes":[{"op":"pass"}]}"#;
        assert!(extract_json(bare).is_ok());
        assert!(extract_json(&format!("```json\n{bare}\n```")).is_ok());
        assert!(extract_json(&format!("Sure! Here you go:\n\n{bare}\n\nEnjoy.")).is_ok());

        // Braces inside a string — shader source is full of them — must not
        // end the object early.
        let shady = r#"{"nodes":[{"op":"pass","params":{"source":"fn f() { return 1; }"}}]}"#;
        let v = extract_json(&format!("here:\n{shady}")).unwrap();
        assert_eq!(v["nodes"][0]["params"]["source"], "fn f() { return 1; }");

        assert!(extract_json("no json at all").is_err());
        assert!(
            extract_json(r#"{"nodes": ["#)
                .unwrap_err()
                .contains("cut off")
        );
    }

    #[test]
    fn an_invented_operator_fails_the_whole_plan() {
        let reg = test_registry();
        let bad = serde_json::json!({
            "nodes": [
                { "name": "a", "op": "pass" },
                { "name": "b", "op": "quantumBlurTOP" }
            ]
        });
        // Not "apply the one that works": half a patch is worse than none.
        let e = parse_plan(&bad, &reg).unwrap_err();
        assert!(e.contains("quantumBlurTOP"), "{e}");
    }

    #[test]
    fn invented_parameters_are_dropped_rather_than_set() {
        let reg = test_registry();
        let value = serde_json::json!({
            "nodes": [{
                "name": "a", "op": "pass",
                "params": { "gain": 2.0, "flux_capacitance": 11 }
            }]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let params = &plan.nodes[0].params;
        assert!(params.contains_key("gain"), "{params:?}");
        assert!(!params.contains_key("flux_capacitance"));
    }

    #[test]
    fn values_are_coerced_to_the_parameters_declared_type() {
        // A float parameter given the integer 1 must become 1.0, and a vec2
        // given a single number must broadcast — both are what a model
        // actually emits.
        assert_eq!(
            json_to_value(&serde_json::json!(1), &Value::Float(0.0)),
            Some(Value::Float(1.0))
        );
        assert_eq!(
            json_to_value(&serde_json::json!(1.03), &Value::Vec2([0.0, 0.0])),
            Some(Value::Vec2([1.03, 1.03]))
        );
        assert_eq!(
            json_to_value(&serde_json::json!([0.1, 0.2, 0.3]), &Value::Vec4([0.0; 4])),
            Some(Value::Vec4([0.1, 0.2, 0.3, 1.0])),
            "a colour given as rgb is opaque"
        );
        assert_eq!(
            json_to_value(&serde_json::json!(1), &Value::Bool(false)),
            Some(Value::Bool(true))
        );
    }

    #[test]
    fn a_plan_builds_nodes_and_wires_them() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let plan = parse_plan(&plan_json(), &reg).unwrap();
        let (applied, viewer) = apply(&mut graph, root, &reg, &plan).unwrap();

        assert_eq!(applied.created, vec!["a", "b"]);
        assert_eq!(applied.wired, 1);
        assert!(applied.warnings.is_empty(), "{:?}", applied.warnings);
        assert_eq!(viewer, graph.find("/b"));
        let b = graph.find("/b").unwrap();
        assert_eq!(graph.node(b).inputs[0], graph.find("/a"));
    }

    #[test]
    fn a_name_that_is_already_taken_is_renamed_and_still_wired() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        // Apply the same plan twice. The second must not fail, must not
        // overwrite the first, and its wire must join *its own* nodes.
        let plan = parse_plan(&plan_json(), &reg).unwrap();
        apply(&mut graph, root, &reg, &plan).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        assert_eq!(applied.wired, 1, "{:?}", applied.warnings);
        assert_ne!(applied.created, vec!["a", "b"], "renamed on collision");
        let second = graph.find(&format!("/{}", applied.created[1])).unwrap();
        let first_of_second = graph.find(&format!("/{}", applied.created[0]));
        assert_eq!(graph.node(second).inputs[0], first_of_second);
    }

    /// `a -> b -> c`, the shape every simplify test needs.
    fn chain(reg: &OpRegistry) -> (Graph, NodeId) {
        let mut graph = Graph::new();
        let root = graph.root();
        let value = serde_json::json!({
            "nodes": [
                { "name": "a", "op": "pass" },
                { "name": "b", "op": "pass" },
                { "name": "c", "op": "pass" }
            ],
            "connections": [
                { "from": "a", "to": "b", "input": 0 },
                { "from": "b", "to": "c", "input": 0 }
            ]
        });
        let plan = parse_plan(&value, reg).unwrap();
        apply(&mut graph, root, reg, &plan).unwrap();
        (graph, root)
    }

    #[test]
    fn set_retunes_an_operator_that_already_exists() {
        // The whole point of `set`: naming it under `nodes` used to get a
        // renamed duplicate beside the original, with the original untouched.
        let reg = test_registry();
        let (mut graph, root) = chain(&reg);
        let value = serde_json::json!({
            "set": [{ "node": "b", "params": { "gain": 0.25, "mode": "add" } }]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        assert!(applied.created.is_empty(), "set creates nothing");
        assert_eq!(applied.changed.len(), 2, "{:?}", applied.warnings);
        let b = graph.find("/b").unwrap();
        assert_eq!(
            graph.node(b).param("gain").unwrap().value,
            Value::Float(0.25)
        );
        // Still three nodes: no duplicate got made.
        assert_eq!(graph.node(root).children.len(), 3);
    }

    #[test]
    fn set_coerces_and_reports_a_parameter_that_is_not_there() {
        let reg = test_registry();
        let (mut graph, root) = chain(&reg);
        let value = serde_json::json!({
            // An int for a float parameter, and one that does not exist.
            "set": [{ "node": "b", "params": { "gain": 2, "warp_factor": 9 } }]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        let b = graph.find("/b").unwrap();
        assert_eq!(
            graph.node(b).param("gain").unwrap().value,
            Value::Float(2.0)
        );
        assert!(
            applied.warnings.iter().any(|w| w.contains("warp_factor")),
            "{:?}",
            applied.warnings
        );
    }

    #[test]
    fn delete_removes_the_node_and_joins_the_chain_back_up() {
        // Deleting from the middle without this leaves `c` with an empty
        // input, which is a black texture and looks like the delete broke it.
        let reg = test_registry();
        let (mut graph, root) = chain(&reg);
        let value = serde_json::json!({ "delete": ["b"] });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        assert_eq!(applied.removed, vec!["b"]);
        assert!(graph.find("/b").is_none());
        let c = graph.find("/c").unwrap();
        assert_eq!(graph.node(c).inputs[0], graph.find("/a"), "chain healed");
    }

    #[test]
    fn deleting_a_run_of_nodes_joins_the_two_surviving_ends() {
        // `a -> b -> c` with both b and c going: nothing to heal onto, but the
        // walk back must not reconnect through a node that is also leaving.
        let reg = test_registry();
        let (mut graph, root) = chain(&reg);
        let extra = serde_json::json!({
            "nodes": [{ "name": "d", "op": "pass" }],
            "connections": [{ "from": "c", "to": "d", "input": 0 }]
        });
        let plan = parse_plan(&extra, &reg).unwrap();
        apply(&mut graph, root, &reg, &plan).unwrap();

        let value = serde_json::json!({ "delete": ["b", "c"] });
        let plan = parse_plan(&value, &reg).unwrap();
        apply(&mut graph, root, &reg, &plan).unwrap();

        let d = graph.find("/d").unwrap();
        assert_eq!(
            graph.node(d).inputs[0],
            graph.find("/a"),
            "d should have been spliced onto a, not left empty or wired to a corpse"
        );
    }

    #[test]
    fn a_plan_may_be_nothing_but_a_change() {
        // "Slow the zoom down" creates nothing. Rejecting it for having no
        // `nodes` sent the model back to invent one it did not need.
        let reg = test_registry();
        let value = serde_json::json!({ "set": [{ "node": "b", "params": { "gain": 0.5 } }] });
        assert!(parse_plan(&value, &reg).is_ok());
        let empty = serde_json::json!({ "notes": "nothing to do" });
        assert!(parse_plan(&empty, &reg).is_err());
    }

    #[test]
    fn every_path_reference_is_rewritten_not_only_feedbacks_target() {
        // The model names another operator before any node has a path, and
        // before the graph has renamed anything that collided. Only `target`
        // used to be rewritten, which left `renderTOP.camera` and friends
        // pointing at a bare name — the entire 3D and instancing half of the
        // program building correctly and rendering an empty frame.
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();

        // Occupy the names first, so the plan's nodes get renamed and a
        // rewrite that merely prepended a slash would point at the wrong one.
        let squatter = parse_plan(
            &serde_json::json!({"nodes":[{"name":"cam1","op":"beat"}]}),
            &reg,
        )
        .unwrap();
        apply(&mut graph, root, &reg, &squatter).unwrap();

        let value = serde_json::json!({
            "nodes": [
                { "name": "cam1", "op": "beat" },
                { "name": "render1", "op": "pass", "params": { "camera": "cam1" } }
            ]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        let render = graph.find_from(root, &applied.created[1]).unwrap();
        let pointed = graph.node(render).param("camera").unwrap().value.as_str();
        assert_eq!(
            pointed,
            format!("/{}", applied.created[0]),
            "should point at the camera this plan made, renamed and all"
        );
    }

    #[test]
    fn an_export_points_a_parameter_at_a_chop_channel() {
        // Export is the only way anything in this program reacts to anything
        // else. Without it in the plan format the assistant could animate on
        // a clock and nothing more: asked to react to a camera it built a
        // patch that looked busy and ignored the room.
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let value = serde_json::json!({
            "nodes": [
                { "name": "lag1", "op": "beat" },
                { "name": "zoom1", "op": "pass", "exports": { "gain": "lag1:chan1" } }
            ]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        assert!(applied.warnings.is_empty(), "{:?}", applied.warnings);
        let zoom = graph.find("/zoom1").unwrap();
        let param = graph.node(zoom).param("gain").unwrap();
        assert_eq!(param.mode, otd_core::param::ParamMode::Expression);
        assert_eq!(param.expression, "ch('/lag1', 'chan1')");
    }

    #[test]
    fn exporting_from_a_top_is_refused_rather_than_silently_reading_nothing() {
        // A parameter pointed at something with no channels does not error at
        // cook time — it just never changes, which looks exactly like a patch
        // that does not react and gives nobody anywhere to look.
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let value = serde_json::json!({
            "nodes": [
                { "name": "src1", "op": "pass" },
                { "name": "zoom1", "op": "pass", "exports": { "gain": "src1:r" } }
            ]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        assert!(
            applied.warnings.iter().any(|w| w.contains("only a CHOP")),
            "{:?}",
            applied.warnings
        );
        let zoom = graph.find("/zoom1").unwrap();
        assert_ne!(
            graph.node(zoom).param("gain").unwrap().mode,
            otd_core::param::ParamMode::Expression
        );
        assert!(
            graph
                .node(zoom)
                .param("gain")
                .unwrap()
                .expression
                .is_empty()
        );
    }

    #[test]
    fn a_malformed_export_target_is_reported() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let value = serde_json::json!({
            "nodes": [{ "name": "a", "op": "pass", "exports": { "gain": "nocolon" } }]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();
        assert!(
            applied.warnings.iter().any(|w| w.contains("node:channel")),
            "{:?}",
            applied.warnings
        );
        // A path with its own colon still splits on the last one.
        assert_eq!(
            split_export("/rack/lag1:chan1"),
            Some(("/rack/lag1", "chan1"))
        );
        assert_eq!(split_export("  lag1 : r "), Some(("lag1", "r")));
        assert_eq!(split_export("lag1:"), None);
    }

    #[test]
    fn a_feedback_loop_closed_with_over_is_warned_about() {
        // The failure that rendering frames found and every structural check
        // missed: source into input 0, feedback into input 1, default `over`.
        // Opaque input 1 wins outright, the source never enters the loop, and
        // the patch fades to black over about a second — long enough to look
        // like it worked.
        let reg = feedback_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let value = serde_json::json!({
            "nodes": [
                { "name": "src1", "op": "pass" },
                { "name": "fb1", "op": "feedback" },
                { "name": "decay1", "op": "pass" },
                { "name": "comp1", "op": "comp" }
            ],
            "connections": [
                { "from": "src1", "to": "comp1", "input": 0 },
                { "from": "fb1", "to": "decay1", "input": 0 },
                { "from": "decay1", "to": "comp1", "input": 1 }
            ]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();
        assert!(
            applied.warnings.iter().any(|w| w.contains("fade to black")),
            "{:?}",
            applied.warnings
        );

        // `add` lets both sides through, so it is not warned about.
        let mut graph = Graph::new();
        let root = graph.root();
        let mut ok = value.clone();
        ok["nodes"][3]["params"] = serde_json::json!({ "operation": "add" });
        let plan = parse_plan(&ok, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();
        assert!(applied.warnings.is_empty(), "{:?}", applied.warnings);
    }

    /// `pass`, plus a two-input composite and a feedback to build a loop from.
    fn feedback_registry() -> OpRegistry {
        fn comp_params() -> IndexMap<String, Param> {
            let mut m = IndexMap::new();
            m.insert(
                "operation".into(),
                Param::menu("over", &["over", "add", "screen"]).with_label("Operation"),
            );
            m
        }
        fn none() -> IndexMap<String, Param> {
            IndexMap::new()
        }
        let mut r = test_registry();
        r.register(OpDef {
            type_name: "comp",
            input_families: &[],
            label: "Comp",
            family: Family::Top,
            inputs: &["base", "over"],
            summary: "",
            time_dependent: false,
            params: comp_params,
            connector: Connector::None,
        });
        r.register(OpDef {
            type_name: "feedback",
            input_families: &[],
            label: "Feedback",
            family: Family::Top,
            inputs: &[],
            summary: "",
            time_dependent: true,
            params: none,
            connector: Connector::None,
        });
        r
    }

    #[test]
    fn deleting_the_whole_network_is_refused() {
        // A model handed deletion can empty a patch on one misread. Clearing
        // the canvas is `/clear` — one word, obviously deliberate.
        let reg = test_registry();
        let (mut graph, root) = chain(&reg);
        let value = serde_json::json!({ "delete": ["a", "b", "c"] });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        assert!(applied.removed.is_empty());
        assert_eq!(graph.node(root).children.len(), 3, "nothing was removed");
        assert!(
            applied.warnings.iter().any(|w| w.contains("/clear")),
            "{:?}",
            applied.warnings
        );
    }

    #[test]
    fn a_node_the_plan_just_created_is_not_deleted_by_it() {
        // A model talking to itself. Honouring it makes the reported node
        // count a lie.
        let reg = test_registry();
        let (mut graph, root) = chain(&reg);
        let value = serde_json::json!({
            "nodes": [{ "name": "fresh", "op": "pass" }],
            "connections": [{ "from": "c", "to": "fresh", "input": 0 }],
            "delete": ["fresh"]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();

        assert!(applied.removed.is_empty(), "{:?}", applied.removed);
        assert!(graph.find("/fresh").is_some());
        assert!(
            applied.warnings.iter().any(|w| w.contains("fresh")),
            "{:?}",
            applied.warnings
        );
    }

    #[test]
    fn delete_cannot_reach_outside_the_network_being_edited() {
        // `resolve` will happily find a path into a component the user is not
        // looking at. A delete they cannot see happen is not one they can
        // catch.
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let outer = parse_plan(
            &serde_json::json!({ "nodes": [{ "name": "keep", "op": "pass" }] }),
            &reg,
        )
        .unwrap();
        apply(&mut graph, root, &reg, &outer).unwrap();

        let value = serde_json::json!({
            "nodes": [{ "name": "new1", "op": "pass" }],
            "delete": ["/keep/nested1", "somewhere_else"]
        });
        let plan = parse_plan(&value, &reg).unwrap();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();
        assert!(applied.removed.is_empty(), "{:?}", applied.removed);
        assert_eq!(applied.warnings.len(), 2, "{:?}", applied.warnings);
    }

    /// The bug behind "the dustbin is on and nothing gets deleted".
    ///
    /// The schema asks for `delete`. Replies also say `deletes`, `remove` and
    /// `removals`, and every one of those used to read as "no removals" —
    /// silently, because a plan that also creates a node is a valid plan, so
    /// it was accepted and applied minus the part the user actually asked for.
    #[test]
    fn a_removal_under_another_name_is_still_a_removal() {
        let reg = test_registry();
        for key in ["delete", "deletes", "remove", "removals"] {
            let json = serde_json::json!({
                "notes": "simplified",
                "nodes": [{ "name": "new1", "op": "pass" }],
                key: ["old1"],
            });
            let plan = parse_plan(&json, &reg).unwrap();
            assert_eq!(
                plan.deletes,
                vec!["old1".to_string()],
                "`{key}` was read as no removals at all"
            );
        }
    }

    /// And the same for the retune list, which fails identically.
    #[test]
    fn a_retune_under_another_name_is_still_a_retune() {
        let reg = test_registry();
        for key in ["set", "sets"] {
            let json = serde_json::json!({
                "notes": "slower",
                key: [{ "node": "a", "params": { "gain": 0.5 } }],
            });
            let plan = parse_plan(&json, &reg).unwrap();
            assert_eq!(plan.sets.len(), 1, "`{key}` was read as no changes at all");
        }
    }

    /// A key the parser cannot guess has to be audible.
    ///
    /// The synonym tables above cover the near misses seen so far. This is
    /// what stops the next one being silent: the plan still applies, but the
    /// user is told something in the reply went unread, rather than being left
    /// to work out why the operator they asked about is still there.
    #[test]
    fn a_key_nobody_reads_is_reported_rather_than_ignored() {
        let reg = test_registry();
        let json = serde_json::json!({
            "notes": "simplified",
            "nodes": [{ "name": "new1", "op": "pass" }],
            "delete_nodes": ["old1"],
        });
        let plan = parse_plan(&json, &reg).unwrap();
        assert!(plan.deletes.is_empty(), "it cannot be guessed, and is not");
        assert!(
            plan.warnings.iter().any(|w| w.contains("delete_nodes")),
            "an unread key said nothing: {:?}",
            plan.warnings
        );

        // And the warning has to survive into what the editor shows.
        let mut graph = Graph::new();
        let root = graph.root();
        let (applied, _) = apply(&mut graph, root, &reg, &plan).unwrap();
        assert!(
            applied.warnings.iter().any(|w| w.contains("delete_nodes")),
            "the parse-time warning never reached the user: {:?}",
            applied.warnings
        );
    }

    /// An empty array is not a mistake, and neither is a key we do read.
    #[test]
    fn a_well_formed_plan_warns_about_nothing() {
        let reg = test_registry();
        let json = serde_json::json!({
            "notes": "a chain",
            "nodes": [{ "name": "a", "op": "pass" }],
            "connections": [],
            "delete": [],
            "viewer": "a",
        });
        let plan = parse_plan(&json, &reg).unwrap();
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    }

    /// "Only change these nodes" has to be a guarantee, not a request.
    ///
    /// The three ways a plan can change an operator that already exists are
    /// filtered when it is out of scope. The fourth thing a plan can do —
    /// read from it — is deliberately left alone, because hanging a new chain
    /// off a locked source is the normal shape of a scoped edit.
    #[test]
    fn a_scoped_plan_cannot_touch_what_was_not_selected() {
        let reg = test_registry();
        let json = serde_json::json!({
            "notes": "worked on the selection",
            "nodes": [{ "name": "new1", "op": "pass" }],
            "connections": [
                // Allowed: reads a locked operator, writes to a selected one.
                { "from": "locked1", "to": "picked1", "input": 0 },
                // Allowed: writes to something this plan just made.
                { "from": "picked1", "to": "new1", "input": 0 },
                // Refused: changes what goes into a locked operator.
                { "from": "new1", "to": "locked1", "input": 0 },
            ],
            "set": [
                { "node": "picked1", "params": { "gain": 0.5 } },
                { "node": "locked1", "params": { "gain": 0.9 } },
            ],
            "delete": ["picked1", "locked1"],
        });
        let mut plan = parse_plan(&json, &reg).unwrap();
        let skipped = gate_scope(&mut plan, &["picked1".to_string()])
            .expect("something was out of scope and that is worth saying");

        assert_eq!(plan.sets.len(), 1);
        assert_eq!(plan.sets[0].node, "picked1");
        assert_eq!(plan.deletes, vec!["picked1".to_string()]);
        assert_eq!(plan.connections.len(), 2, "only the inbound wire goes");
        assert!(
            plan.connections.iter().all(|w| w.to != "locked1"),
            "something is still wired into a locked operator"
        );
        assert!(
            plan.connections.iter().any(|w| w.from == "locked1"),
            "reading from a locked operator should still be allowed"
        );
        // And the user is told, by name, what did not happen.
        assert!(skipped.contains("locked1"), "{skipped}");
        assert!(!skipped.contains("picked1"), "{skipped}");
    }

    /// No selection means no scoping — not a locked-down patch.
    #[test]
    fn an_empty_scope_filters_nothing() {
        let reg = test_registry();
        let json = serde_json::json!({
            "set": [{ "node": "anything1", "params": { "gain": 0.5 } }],
            "delete": ["whatever1"],
        });
        let mut plan = parse_plan(&json, &reg).unwrap();
        let before = plan.clone();
        assert_eq!(gate_scope(&mut plan, &[]), None);
        assert_eq!(plan, before);
    }

    /// The model is shown which operators it may touch, per operator.
    #[test]
    fn a_description_marks_what_is_in_scope_and_what_is_locked() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let plan = parse_plan(&plan_json(), &reg).unwrap();
        apply(&mut graph, root, &reg, &plan).unwrap();
        let a = graph.find_from(root, "a").unwrap();

        let scoped = describe(&graph, root, None, None, &[a], &reg);
        let line = |name: &str| {
            scoped
                .lines()
                .find(|l| l.starts_with(&format!("- {name} ")))
                .unwrap_or_else(|| panic!("no line for {name} in:\n{scoped}"))
                .to_string()
        };
        assert!(line("a").ends_with("[IN SCOPE]"), "{}", line("a"));
        assert!(line("b").ends_with("[LOCKED]"), "{}", line("b"));

        // With nothing scoped, neither mark appears — an unscoped request
        // should not read as though everything were locked.
        let open = describe(&graph, root, None, None, &[], &reg);
        assert!(!open.contains("[IN SCOPE]"), "{open}");
        assert!(!open.contains("[LOCKED]"), "{open}");
    }

    #[test]
    fn the_catalogue_is_generated_from_the_registry() {
        let reg = test_registry();
        let text = catalogue(&reg);
        for def in reg.iter() {
            assert!(text.contains(def.type_name), "{} missing", def.type_name);
        }
        // And it lists parameters, or the model has to guess them.
        assert!(text.contains("gain"), "{text}");
        // And what each operator is FOR. A catalogue of names and parameters
        // tells a model how to spell an operator, not when to reach for it.
        for def in reg.iter() {
            assert!(
                def.summary.is_empty() || text.contains(def.summary),
                "{} has no summary in the catalogue",
                def.type_name
            );
        }
    }

    #[test]
    fn describing_a_network_lists_what_is_wired_to_what() {
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        assert!(describe(&graph, root, None, None, &[], &reg).contains("empty"));

        let plan = parse_plan(&plan_json(), &reg).unwrap();
        apply(&mut graph, root, &reg, &plan).unwrap();
        let text = describe(&graph, root, None, None, &[], &reg);
        assert!(text.contains("a (pass)"), "{text}");
        assert!(text.contains("0<-a"), "{text}");
    }

    #[test]
    fn a_description_carries_what_was_tuned_and_what_is_selected() {
        // Building on a patch means seeing it. A model that cannot read the
        // numbers already dialled in proposes its own over the top of them,
        // and one that cannot tell which node is selected has nothing to
        // attach "make it move" to.
        let reg = test_registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let plan = parse_plan(&plan_json(), &reg).unwrap();
        apply(&mut graph, root, &reg, &plan).unwrap();

        let tuned = graph.find_from(root, "a").unwrap();
        graph.set_param(tuned, "gain", Value::Float(0.35)).unwrap();
        let text = describe(&graph, root, Some(tuned), None, &[], &reg);

        assert!(text.contains("gain=0.35"), "{text}");
        assert!(text.contains("[SELECTED"), "{text}");
        // Positions are what keep a new chain off the top of an old one.
        assert!(text.contains(" at ["), "{text}");
        // A default is in the catalogue already; repeating it is prompt
        // spent to say nothing.
        let untouched = describe(&graph, root, None, None, &[], &reg);
        assert!(!untouched.contains("gain=1"), "{untouched}");
    }

    #[test]
    fn a_shader_that_broke_the_json_is_reported_rather_than_swallowed() {
        // Both ways a model mangles JSON do it in the same place: a long
        // shader source, where an escape gets dropped. These have to come
        // back as errors, because the error is what gets handed to the model
        // to fix — see `complete_with_repair`. A parse that quietly returned
        // something half-read would skip the retry entirely.
        let unescaped_quote = "{\"nodes\":[{\"params\":{\"source\":\"// uses \"iTime\"\"}}]}";
        let err = extract_json(unescaped_quote).unwrap_err();
        assert!(err.contains("not valid JSON"), "{err}");

        let literal_newline =
            "{\"nodes\":[{\"params\":{\"source\":\"void main() {\nreturn;\n}\"}}]}";
        let err = extract_json(literal_newline).unwrap_err();
        assert!(err.contains("not valid JSON"), "{err}");

        // Properly escaped, the same shader is fine — the brace scanner must
        // not be fooled by the braces inside the string.
        let good = "{\"nodes\":[{\"params\":{\"source\":\"void main() {\\n  return;\\n}\"}}]}";
        assert!(extract_json(good).is_ok());
    }

    #[test]
    fn long_text_is_clipped_rather_than_dropped() {
        // Knowing a node already has a shader is what stops the model
        // writing a second one — but the whole body would crowd out the
        // patch it is meant to be extending.
        let long = "x".repeat(400);
        let out = clip(&long);
        assert!(out.contains("400 chars"), "{out}");
        assert!(out.chars().count() < 140, "{out}");
        // Short values pass through untouched.
        assert_eq!(clip("noise1"), "noise1");
    }
