use super::*;

    fn argv(s: &str) -> Vec<String> {
        std::iter::once("otd".to_string())
            .chain(s.split_whitespace().map(String::from))
            .collect()
    }

    #[test]
    fn flags_are_parsed_in_any_order() {
        let a = parse(argv("render p.otd --out shots --frames 30 --node /out1")).unwrap();
        assert_eq!(a.command, "render");
        assert_eq!(a.project, PathBuf::from("p.otd"));
        assert_eq!(a.frames, Some(30));
        assert_eq!(a.node.as_deref(), Some("/out1"));
        assert_eq!(a.out, PathBuf::from("shots"));
    }

    #[test]
    fn defaults_do_not_silently_pick_a_frame_rate() {
        // Left alone, the frame rate comes from the project rather than from
        // a number invented here.
        let a = parse(argv("run p.otd")).unwrap();
        assert_eq!(a.fps, None);
        assert_eq!(a.frames, None, "`run` runs until interrupted");
    }

    #[test]
    fn a_bad_argument_says_what_was_wrong() {
        for (args, wanted) in [
            ("render p.otd", "--node"),
            ("run p.otd --frames soon", "not a whole number"),
            ("run p.otd --fps 0", "greater than zero"),
            ("run p.otd --fps", "needs a value"),
            ("dance p.otd", "unknown command"),
            ("run", "needs a project file"),
            ("run p.otd --loud", "unknown option"),
        ] {
            let Err(Fail::Err(err)) = parse(argv(args)) else {
                panic!("`{args}` should have been rejected");
            };
            assert!(err.contains(wanted), "`{args}` -> {err}");
        }
    }

    #[test]
    fn asking_for_help_is_not_a_failure() {
        // `otd --help` in a script must not exit non-zero, and it must not
        // complain about a project file it was never given.
        for args in ["--help", "-h", "help", ""] {
            assert!(
                matches!(parse(argv(args)), Err(Fail::Help)),
                "`otd {args}` should print usage and succeed"
            );
        }
    }
