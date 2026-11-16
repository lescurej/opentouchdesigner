use super::*;
use otd_core::Graph;
use otd_engine::registry;

    #[test]
    fn analyze_fixture_cparm_line() {
        let line = "1 Label label 1 1 0 0 1 1 1 2 0 \"\" \"\" Custom 0";
        let (key, p) = parse_cparm_line(line).expect("line");
        assert_eq!(key, "Label");
        assert_eq!(p.label, "label");
        assert!((p.value.as_f64() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn slackermanz_cparm_fixture() {
        let path = "/Users/johanlescure/Desktop/Projets/Parreno/Sydney/Automaton/GoF.tox.dir/project1/slackermanz2.cparm";
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let mut g = Graph::new();
        let comp = g
            .create(g.root(), registry().get("baseCOMP").unwrap(), Some("slackermanz2"))
            .unwrap();
        apply_cparm_block(&mut g, comp, &text);
        assert!(g.node(comp).params.contains_key("Seed"));
        assert!(g.node(comp).params.contains_key("Pausesim"));
        assert!(g.node(comp).params.contains_key("Displaymode"));
        assert!(g.node(comp).params.contains_key("Rulepreset"));
        let disp = &g.node(comp).params["Displaymode"];
        assert!(disp.menu.is_some());
        assert_eq!(disp.value.as_str(), "0");
    }

    #[test]
    fn protomarquee_cparm_fixture() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protomarquee.tox.dir/protomarquee.cparm"
        );
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        let mut g = Graph::new();
        let comp = g
            .create(g.root(), registry().get("baseCOMP").unwrap(), Some("x"))
            .unwrap();
        apply_cparm_block(&mut g, comp, &text);
        assert!(g.node(comp).params.contains_key("Testmode"));
        assert!(g.node(comp).params["Testmode"].value.as_bool());
    }
