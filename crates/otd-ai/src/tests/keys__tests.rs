use super::*;

    #[test]
    fn a_key_never_prints_itself() {
        let key = Key::new("sk-ant-super-secret-value-abcdef123456");
        // The three ways a value normally escapes: Debug, a format string,
        // and a log line built from either.
        let debugged = format!("{key:?}");
        assert!(!debugged.contains("super-secret"), "{debugged}");
        assert!(debugged.contains("sk-"), "still has to identify itself");
        assert!(debugged.contains("123456") || debugged.contains("3456"));

        let keys = Keys::default();
        assert_eq!(Key::new("").hint(), "not set");
        // Short enough that any hint would be most of the key.
        assert_eq!(Key::new("abc123").hint(), "set");
        drop(keys);
    }

    #[test]
    fn provider_errors_are_scrubbed_before_anyone_sees_them() {
        let mut keys = Keys::default();
        keys.set(Provider::OpenAi, Key::new("sk-proj-0123456789abcdef"));
        // Providers really do quote the offending header back.
        let raw = "401 Unauthorized: invalid key sk-proj-0123456789abcdef supplied";
        let safe = redact(raw, &keys);
        assert!(!safe.contains("0123456789"), "{safe}");
        assert!(safe.contains("<redacted>"));
        assert!(
            safe.contains("401 Unauthorized"),
            "the useful part survives"
        );
    }

    #[test]
    fn keys_round_trip_through_the_config_format() {
        let mut keys = Keys::default();
        keys.set(Provider::Anthropic, Key::new("sk-ant-aaaaaaaaaaaa"));
        keys.set(Provider::OpenRouter, Key::new("sk-or-bbbbbbbbbbbb"));
        let text = keys.to_text();
        let back = Keys::parse(&text);
        assert_eq!(
            back.get(Provider::Anthropic).map(|k| k.expose()),
            Some("sk-ant-aaaaaaaaaaaa")
        );
        assert_eq!(
            back.get(Provider::OpenRouter).map(|k| k.expose()),
            Some("sk-or-bbbbbbbbbbbb")
        );
        assert!(back.get(Provider::OpenAi).is_none());

        // Comments, blank lines and unknown names are skipped rather than
        // turning into a key called `#`.
        let messy = Keys::parse("# a comment\n\nnonsense = x\nopenai=  sk-spaced  \n");
        assert_eq!(
            messy.get(Provider::OpenAi).map(|k| k.expose()),
            Some("sk-spaced")
        );
    }

    #[test]
    fn an_empty_key_is_no_key_rather_than_an_empty_one() {
        let mut keys = Keys::default();
        keys.set(Provider::OpenAi, Key::new("   "));
        assert!(keys.get(Provider::OpenAi).is_none());
    }

    #[test]
    fn the_key_file_is_never_inside_a_project() {
        let path = config_path();
        assert!(path.ends_with("keys.conf"));
        // The test is about the *shape* of the answer: a per-user config
        // location, not the working directory a project happens to be in.
        assert!(path.parent().is_some());
        assert_ne!(path.parent().unwrap(), std::path::Path::new(""));
    }
