use std::collections::BTreeMap;

use super::*;

    /// Stands in for naga: anything mentioning `ghost` is undefined.
    fn fake_check(source: &str, _is_glsl: bool) -> Result<(), String> {
        if source.contains("ghost") {
            Err("no definition in scope for `ghost`".into())
        } else {
            Ok(())
        }
    }

    #[test]
    fn a_shader_that_will_not_compile_is_found_before_the_node_exists() {
        // The failure this catches is the nasty one: the patch looks built,
        // the node outlines red, and the output is silently black.
        let value = serde_json::json!({
            "nodes": [
                { "name": "zig1", "op": "glslTOP",
                  "params": { "source": "return vec4f(ghost, 0.0, 0.0, 1.0);" } },
                { "name": "ok1", "op": "glslTOP",
                  "params": { "source": "return vec4f(1.0);" } }
            ]
        });
        let problems = shader_problems(&value, fake_check);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].0, "zig1");
        assert!(problems[0].1.contains("ghost"));
    }

    #[test]
    fn nodes_without_shaders_are_not_checked() {
        let value = serde_json::json!({
            "nodes": [
                { "name": "a", "op": "noiseTOP", "params": { "period": 0.3 } },
                { "name": "b", "op": "glslTOP", "params": { "source": "  " } },
                { "name": "c", "op": "textDAT", "params": { "text": "ghost" } }
            ]
        });
        // `c` has a `text` parameter, not a `source` one — checking every
        // string a model wrote would reject prose.
        assert!(shader_problems(&value, fake_check).is_empty());
    }

    #[test]
    fn a_stray_source_param_on_a_non_shader_operator_is_not_checked() {
        // Seen in the wild: the model hangs a `source` on a levelTOP, the
        // string is compiled as WGSL, and a syntax error nobody can act on
        // costs a repair round trip — the model quite correctly answers that
        // the patch has no glslTOP in it. `apply` drops the parameter anyway.
        let value = serde_json::json!({
            "nodes": [
                { "name": "threshold1", "op": "levelTOP",
                  "params": { "source": "ghost" } },
                { "name": "callbacks1", "op": "executeDAT",
                  "params": { "source": "def onCook(dat):\n\tghost()\n" } }
            ]
        });
        assert!(shader_problems(&value, fake_check).is_empty());
    }

    #[test]
    fn a_chain_the_model_forgot_to_join_on_is_reported() {
        // Exactly what a model does when asked to add to an existing patch:
        // builds a whole second loop and never wires it in.
        let plan = Plan {
            nodes: vec![
                node("a", "noiseTOP"),
                node("b", "levelTOP"),
                node("out1", "nullTOP"),
                node("fb2", "feedbackTOP"),
                node("decay2", "levelTOP"),
            ],
            connections: vec![wire("a", "b"), wire("b", "out1"), wire("fb2", "decay2")],
            viewer: Some("out1".into()),
            ..Default::default()
        };
        let loose = dangling(&plan);
        // `decay2` feeds nothing and is not the output. `fb2` feeds decay2,
        // and `out1` is the viewer, so neither is loose.
        assert_eq!(loose, vec!["decay2"]);
    }

    #[test]
    fn a_feedback_node_is_never_called_loose() {
        // Feedback is read through its `target` parameter, not a wire, so
        // the usual "nothing reads it" test would libel it.
        let plan = Plan {
            nodes: vec![node("fb1", "feedbackTOP")],
            ..Default::default()
        };
        assert!(dangling(&plan).is_empty());
    }

    fn node(name: &str, op: &str) -> PlannedNode {
        PlannedNode {
            name: name.into(),
            op: op.into(),
            pos: [0.0, 0.0],
            params: BTreeMap::new(),
            expressions: BTreeMap::new(),
            exports: BTreeMap::new(),
        }
    }

    fn wire(from: &str, to: &str) -> PlannedWire {
        PlannedWire {
            from: from.into(),
            to: to.into(),
            input: 0,
        }
    }
