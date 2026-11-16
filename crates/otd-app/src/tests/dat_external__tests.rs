use super::*;

    #[test]
    fn default_editor_is_nonempty() {
        assert!(!default_editor_command().trim().is_empty());
    }

    #[test]
    fn normalize_fixes_common_path_placeholder_typos() {
        assert_eq!(
            normalize_path_placeholder("open -a Foo %path¨%"),
            "open -a Foo %path%"
        );
    }

    #[test]
    fn shell_escape_quotes_paths() {
        assert_eq!(shell_escape_posix("/tmp/a b"), "'/tmp/a b'");
    }
