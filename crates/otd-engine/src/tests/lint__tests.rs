use super::*;
    use otd_core::Graph;

    #[test]
    fn flags_missing_op_path() {
        let graph = Graph::new();
        let issues = lint_op_paths(&graph, "/project1", "x = op('nope')");
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("nope"));
    }

    #[test]
    fn flags_missing_ch_op() {
        let graph = Graph::new();
        let issues = lint_ch_paths(&graph, "/project1", "x = ch('nope', 'chan1')", |_| vec![]);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("nope"));
    }

    #[test]
    fn lint_does_not_panic_on_utf8_in_comments() {
        let graph = Graph::new();
        let src = "# café à la mode\nx = op('missing')\n";
        let issues = lint_op_paths(&graph, "/project1", src);
        assert_eq!(issues.len(), 1);
    }
