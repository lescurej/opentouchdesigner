use super::*;

use super::*;

    #[test]
    fn expression_mode_keeps_the_constant_it_replaced() {
        let mut p = Param::float(0.25);
        p.set_expression("absTime * 2");
        assert_eq!(p.mode, ParamMode::Expression);
        let v = p.eval(&EvalContext {
            abs_time: 1.5,
            ..Default::default()
        });
        assert_eq!(v.as_f64(), 3.0);

        p.mode = ParamMode::Constant;
        assert_eq!(p.eval(&EvalContext::default()).as_f64(), 0.25);
    }

    #[test]
    fn an_expression_the_fast_path_cannot_parse_is_handed_to_python() {
        // Anything the built-in language does not understand is Python's,
        // whether it is valid Python or not — this crate cannot tell, and
        // guessing would reject working code.
        let mut p = Param::float(7.0);
        p.set_expression("sum(x for x in range(4))");
        assert!(p.needs_python);
        assert!(
            p.is_time_dependent(),
            "an unanalysable expression must be assumed animated"
        );
        assert_eq!(p.referenced_ops().count(), 0);

        // With no interpreter available the constant stands, and asking for
        // the value explicitly says why.
        assert_eq!(p.eval(&EvalContext::default()).as_f64(), 7.0);
        assert!(
            p.eval_checked(&EvalContext::default())
                .unwrap_err()
                .contains("Python")
        );
    }

    #[test]
    fn paths_mentioned_in_an_expression_become_references() {
        let mut p = Param::float(0.0);
        p.set_expression("ch('/lfo1', 'chan1') + ch(\"/audio/level\", 'band1')");
        let refs: Vec<&str> = p.referenced_ops().collect();
        assert_eq!(refs, vec!["/lfo1", "/audio/level"]);

        // A quoted string that is not a path is not a reference.
        p.set_expression("'hello'.upper()");
        assert_eq!(p.referenced_ops().count(), 0);
    }

    struct FakeNetwork;
    impl crate::expr::ChannelSource for FakeNetwork {
        fn channel(&self, op_path: &str, channel: &str) -> Option<f32> {
            (op_path == "/lfo1" && channel == "chan1").then_some(0.75)
        }
        fn param_value(&self, op_path: &str, param: &str) -> Option<Value> {
            (op_path == "/other" && param == "size").then_some(Value::Float(12.0))
        }
        fn parent_param(&self, _node_path: &str, param: &str) -> Option<Value> {
            (param == "label").then_some(Value::Str("Go".into()))
        }
    }

    fn ctx_with_network() -> EvalContext<'static> {
        EvalContext {
            channels: Some(&FakeNetwork),
            ..Default::default()
        }
    }

    #[test]
    fn export_mode_reads_a_chop_channel() {
        let mut p = Param::float(0.1);
        p.set_export("/lfo1", "chan1");
        assert_eq!(p.mode, ParamMode::Export);
        assert_eq!(p.source_op(), Some("/lfo1"));
        assert_eq!(p.eval(&ctx_with_network()).as_f64(), 0.75);
        // With no network to read from, the constant stands in.
        assert_eq!(p.eval(&EvalContext::default()).as_f64(), 0.1);
    }

    #[test]
    fn bind_mode_reads_another_parameter() {
        let mut p = Param::float(1.0);
        p.set_bind("/other", "size");
        assert_eq!(p.eval(&ctx_with_network()).as_f64(), 12.0);
    }

    #[test]
    fn a_bind_of_the_wrong_type_is_ignored() {
        let mut p = Param::str("hello");
        p.set_bind("/other", "size");
        assert_eq!(p.eval(&ctx_with_network()).as_str(), "hello");
    }

    #[test]
    fn a_dangling_export_falls_back_to_the_constant() {
        let mut p = Param::float(0.4);
        p.set_export("/gone", "chan1");
        assert_eq!(p.eval(&ctx_with_network()).as_f64(), 0.4);
    }

    #[test]
    fn int_parameters_stay_ints_under_expressions() {
        let mut p = Param::int(0);
        p.set_expression("2.6");
        assert_eq!(p.eval(&EvalContext::default()), Value::Int(3));
    }

    #[test]
    fn parent_label_expression_on_string_param() {
        let mut p = Param::str("Button");
        p.set_expression("parent.label");
        let ctx = EvalContext {
            channels: Some(&FakeNetwork),
            path: Some("/button1/text"),
            ..Default::default()
        };
        assert_eq!(p.eval(&ctx), Value::Str("Go".into()));
        assert_eq!(p.eval_checked(&ctx).unwrap(), Value::Str("Go".into()));
    }
