use super::*;

use super::*;

    fn ctx_with<'a>(locals: &'a [(&'a str, f64)]) -> EvalContext<'a> {
        EvalContext {
            frame: 0,
            time: 0.0,
            abs_time: 0.0,
            fps: 60.0,
            channels: None,
            path: None,
            locals,
        }
    }

    #[test]
    fn operator_supplied_names_are_visible_to_an_expression() {
        let locals = [("v", 0.5), ("i", 9.0), ("n", 10.0)];
        let ctx = ctx_with(&locals);
        let got = Expr::parse("i / (n - 1)").unwrap().eval(&ctx).unwrap();
        assert_eq!(got, 1.0);
        assert_eq!(Expr::parse("v * 2").unwrap().eval(&ctx).unwrap(), 1.0);
    }

    /// `-` is one token doing two jobs, and the binary one was never wired to
    /// the operator loop: every expression with a subtraction in it failed to
    /// parse, which in a parameter means silently keeping the constant.
    #[test]
    fn subtraction_is_an_operator_and_not_only_a_sign() {
        let ctx = ctx_with(&[]);
        for (src, want) in [
            ("2 - 1", 1.0),
            ("(2 - 1)", 1.0),
            ("10 - 2 - 3", 5.0),
            ("2 - -3", 5.0),
            ("-2 - 3", -5.0),
            ("2 * 3 - 1", 5.0),
            ("2 - 3 * 2", -4.0),
            ("max(5 - 1, 0)", 4.0),
        ] {
            let got = Expr::parse(src)
                .unwrap_or_else(|e| panic!("`{src}` did not parse: {e:?}"))
                .eval(&ctx)
                .unwrap_or_else(|e| panic!("`{src}` did not evaluate: {e:?}"));
            assert_eq!(got, want, "`{src}`");
        }
    }

    #[test]
    fn f_strings_are_not_handled_by_the_fast_path() {
        let s = "f'[{parent(2).par.Dmxstartaddress}-{parent(2).par.Dmxstartaddress+20}]'";
        assert!(
            Expr::parse(s).is_err(),
            "TD f-strings must go through Python"
        );
    }

    #[test]
    fn a_subtraction_still_propagates_time_dependence() {
        // It parses as a binary node now, so the walk that decides whether a
        // parameter re-cooks every frame has to reach through it.
        assert!(Expr::parse("absTime - 1").unwrap().is_time_dependent());
        assert!(!Expr::parse("3 - 1").unwrap().is_time_dependent());
    }
