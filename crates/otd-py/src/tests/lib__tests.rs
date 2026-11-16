use super::*;
    use otd_core::ChannelSource;

    struct FakeNet;
    impl ChannelSource for FakeNet {
        fn channel(&self, op_path: &str, channel: &str) -> Option<f32> {
            (op_path == "/lfo1" && channel == "chan1").then_some(0.5)
        }
        fn param_value(&self, op_path: &str, name: &str) -> Option<Value> {
            (op_path == "/blur1" && name == "size").then_some(Value::Float(8.0))
        }
        fn parent_param(&self, _node: &str, name: &str) -> Option<Value> {
            (name == "speed").then_some(Value::Float(2.5))
        }
        fn resolve_operator_path(&self, _from: &str, reference: &str) -> Option<String> {
            match reference {
                "text1" => Some("/text1".into()),
                "table1" => Some("/table1".into()),
                p if p.starts_with('/') => Some(p.into()),
                _ => None,
            }
        }
        fn dat_text(&self, op_path: &str) -> Option<String> {
            (op_path == "/text1").then(|| "from the dat".into())
        }
        fn dat_rows(&self, op_path: &str) -> Option<Vec<Vec<String>>> {
            (op_path == "/table1").then(|| {
                vec![
                    vec!["a".into(), "b".into()],
                    vec!["1".into(), "2".into()],
                ]
            })
        }
    }

    fn ctx() -> EvalContext<'static> {
        EvalContext {
            abs_time: 2.0,
            frame: 120,
            fps: 60.0,
            channels: Some(&FakeNet),
            ..Default::default()
        }
    }

    #[test]
    fn arithmetic_and_the_maths_scope() {
        let mut e = PyEngine::new();
        assert!(e.is_available(), "{:?}", e.startup_error);
        assert_eq!(e.eval("1 + 2 * 3", &ctx(), "/x").unwrap().as_f64(), 7.0);
        assert!((e.eval("sin(pi / 2)", &ctx(), "/x").unwrap().as_f64() - 1.0).abs() < 1e-12);
        assert_eq!(
            e.eval("clamp(5, 0, 1)", &ctx(), "/x").unwrap().as_f64(),
            1.0
        );
        assert_eq!(
            e.eval("fit(0.5, 0, 1, 10, 20)", &ctx(), "/x")
                .unwrap()
                .as_f64(),
            15.0
        );
    }

    #[test]
    fn time_and_the_network_are_in_scope() {
        let mut e = PyEngine::new();
        assert_eq!(e.eval("absTime", &ctx(), "/x").unwrap().as_f64(), 2.0);
        assert_eq!(e.eval("frame", &ctx(), "/x").unwrap().as_i64(), 120);
        assert_eq!(
            e.eval("ch('/lfo1', 'chan1')", &ctx(), "/x")
                .unwrap()
                .as_f64(),
            0.5
        );
        assert_eq!(
            e.eval("par('/blur1', 'size')", &ctx(), "/x")
                .unwrap()
                .as_f64(),
            8.0
        );
        assert_eq!(
            e.eval("parent('speed')", &ctx(), "/x").unwrap().as_f64(),
            2.5
        );
    }

    #[test]
    fn python_expressions_can_use_python() {
        let mut e = PyEngine::new();
        assert_eq!(
            e.eval("sum(x * x for x in range(4))", &ctx(), "/x")
                .unwrap()
                .as_f64(),
            14.0
        );
        assert_eq!(
            e.eval("'on' if absTime > 1 else 'off'", &ctx(), "/x")
                .unwrap()
                .as_str(),
            "on"
        );
        assert_eq!(
            e.eval("[1.0, 2.0, 3.0]", &ctx(), "/x").unwrap(),
            Value::Vec3([1.0, 2.0, 3.0])
        );
    }

    #[test]
    fn op_text_reads_dat_contents() {
        let mut e = PyEngine::new();
        assert_eq!(
            e.eval("op('text1').text", &ctx(), "/webserver1")
                .unwrap()
                .as_str(),
            "from the dat"
        );
    }

    #[test]
    fn op_dat_table_cells() {
        let mut e = PyEngine::new();
        assert_eq!(
            e.eval("op('table1').numRows", &ctx(), "/x")
                .unwrap()
                .as_i64(),
            2
        );
        assert_eq!(
            e.eval("op('table1').getCell(1, 1)", &ctx(), "/x")
                .unwrap()
                .as_str(),
            "2"
        );
        assert_eq!(
            e.eval("op('table1')[1, 1]", &ctx(), "/x").unwrap().as_str(),
            "2"
        );
    }

    #[test]
    fn an_error_is_reported_as_one_line() {
        let mut e = PyEngine::new();
        let err = e.eval("1 / 0", &ctx(), "/x").unwrap_err();
        assert!(err.contains("ZeroDivisionError"), "{err}");
        assert!(err.len() < 200, "{err}");

        let err = e.eval("nope +", &ctx(), "/x").unwrap_err();
        assert!(err.contains("SyntaxError"), "{err}");

        let err = e.eval("undefined_name", &ctx(), "/x").unwrap_err();
        assert!(err.contains("NameError"), "{err}");
    }

    #[test]
    fn the_network_is_unreachable_once_evaluation_ends() {
        let mut e = PyEngine::new();
        // Stash the function, then call it outside any evaluation: it must
        // read nothing rather than dereference a stale pointer.
        e.eval("ch('/lfo1', 'chan1')", &ctx(), "/x").unwrap();
        let no_net = EvalContext {
            channels: None,
            ..Default::default()
        };
        assert_eq!(
            e.eval("ch('/lfo1', 'chan1')", &no_net, "/x")
                .unwrap()
                .as_f64(),
            0.0
        );
    }

    #[test]
    fn a_script_can_return_a_value_and_a_table() {
        let mut e = PyEngine::new();
        let v = e
            .run(
                "total = 0\nfor i in range(5):\n    total += i\nresult = total",
                &ctx(),
                "/x",
            )
            .unwrap();
        assert_eq!(v.map(|v| v.as_f64()), Some(10.0));

        let rows = e
            .run_table(
                "rows = [['name', 'value'], ['a', 1], ['b', 2]]",
                &ctx(),
                "/x",
            )
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["name", "value"]);
        assert_eq!(rows[2][1], "2");
    }

    #[test]
    fn compiling_the_same_source_twice_reuses_it() {
        let mut e = PyEngine::new();
        for _ in 0..100 {
            e.eval("absTime * 2", &ctx(), "/x").unwrap();
        }
    }
