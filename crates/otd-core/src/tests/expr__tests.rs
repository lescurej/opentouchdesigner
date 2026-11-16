use super::*;

    fn ev(src: &str) -> f64 {
        Expr::parse(src)
            .expect("parse")
            .eval(&EvalContext {
                frame: 60,
                time: 1.0,
                abs_time: 2.0,
                fps: 60.0,
                channels: None,
                path: None,
                locals: &[],
            })
            .expect("eval")
    }

    #[test]
    fn arithmetic_precedence() {
        assert_eq!(ev("1 + 2 * 3"), 7.0);
        assert_eq!(ev("(1 + 2) * 3"), 9.0);
        assert_eq!(ev("2 ** 3 ** 2"), 512.0); // right associative
        assert_eq!(ev("-2 + 3"), 1.0);
        assert_eq!(ev("10 % 3"), 1.0);
    }

    #[test]
    fn functions_and_vars() {
        assert!((ev("sin(pi/2)") - 1.0).abs() < 1e-12);
        assert_eq!(ev("clamp(5, 0, 1)"), 1.0);
        assert_eq!(ev("fit(0.5, 0, 1, 10, 20)"), 15.0);
        assert_eq!(ev("frame"), 60.0);
        assert_eq!(ev("absTime"), 2.0);
    }

    #[test]
    fn time_dependence_is_detected() {
        assert!(Expr::parse("sin(absTime * 2)").unwrap().is_time_dependent());
        assert!(Expr::parse("frame % 30").unwrap().is_time_dependent());
        assert!(!Expr::parse("1 + 2 * pi").unwrap().is_time_dependent());
        assert!(
            Expr::parse("ch('/lfo1', 'chan1')")
                .unwrap()
                .is_time_dependent()
        );
    }

    struct FakeNet;
    impl ChannelSource for FakeNet {
        fn channel(&self, op_path: &str, channel: &str) -> Option<f32> {
            matches!(
                (op_path, channel),
                ("/lfo1", "chan1") | ("/geo1/lfo1", "chan1")
            )
            .then_some(0.25)
        }
        fn param_value(&self, op_path: &str, name: &str) -> Option<crate::Value> {
            (op_path == "/blur1" && name == "size").then_some(crate::Value::Float(8.0))
        }
    }

    #[test]
    fn ch_and_par_read_the_network() {
        let net = FakeNet;
        let ctx = EvalContext {
            frame: 0,
            time: 0.0,
            abs_time: 0.0,
            fps: 60.0,
            channels: Some(&net),
            path: Some("/geo1/noise1"),
            locals: &[],
        };
        let v = Expr::parse("ch('/lfo1', 'chan1')")
            .unwrap()
            .eval(&ctx)
            .unwrap();
        assert!((v - 0.25).abs() < 1e-6);
        let v = Expr::parse("ch('lfo1', 'chan1')")
            .unwrap()
            .eval(&ctx)
            .unwrap();
        assert!((v - 0.25).abs() < 1e-6);
        let v = Expr::parse("op('lfo1')['chan1']")
            .unwrap()
            .eval(&ctx)
            .unwrap();
        assert!((v - 0.25).abs() < 1e-6);
        let refs = Expr::parse("op('lfo1')['chan1']")
            .unwrap()
            .referenced_paths();
        assert_eq!(refs, vec!["lfo1"]);
        let p = Expr::parse("par('/blur1', 'size')")
            .unwrap()
            .eval(&ctx)
            .unwrap();
        assert!((p - 8.0).abs() < 1e-6);
    }

    #[test]
    fn errors_are_reported_not_panicked() {
        assert!(Expr::parse("1 +").is_err());
        assert!(Expr::parse("foo(").is_err());
        assert!(Expr::parse("2 $ 3").is_err());
        assert!(
            Expr::parse("nope")
                .unwrap()
                .eval(&EvalContext::default())
                .is_err()
        );
    }
