use super::*;
use crate::test_ops::{TEST_PASS, registry};
use crate::Value;

    fn two_chain() -> (Graph, NodeId, NodeId) {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let a = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        let b = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        g.connect(a, b, 0).unwrap();
        (g, a, b)
    }

    #[test]
    fn names_are_auto_numbered() {
        let (g, a, b) = two_chain();
        assert_eq!(g.node(a).name, "pass1");
        assert_eq!(g.node(b).name, "pass2");
        assert_eq!(g.path(b), "/pass2");
        assert_eq!(g.find("/pass2"), Some(b));
    }

    #[test]
    fn an_explicit_name_is_kept_exactly() {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let a = g
            .create(root, reg.get("pass").unwrap(), Some("myBlur"))
            .unwrap();
        assert_eq!(g.node(a).name, "myBlur");
        // ...unless it collides, in which case it is numbered.
        let b = g
            .create(root, reg.get("pass").unwrap(), Some("myBlur"))
            .unwrap();
        assert_eq!(g.node(b).name, "myBlur1");
    }

    #[test]
    fn cycles_are_rejected() {
        let (mut g, a, b) = two_chain();
        assert_eq!(g.connect(b, a, 0), Err(GraphError::WouldCycle));
        assert_eq!(g.connect(a, a, 0), Err(GraphError::WouldCycle));
    }

    #[test]
    fn channel_expression_links_export_and_ch() {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let lfo = g.create(root, reg.get("chop_pass").unwrap(), None).unwrap();
        let lfo_path = g.path(lfo);
        let lfo_name = g.node(lfo).name.clone();
        let top = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        g.node_mut(top)
            .params
            .get_mut(TEST_PASS)
            .unwrap()
            .set_export(&lfo_path, "chan1");
        assert_eq!(g.channel_expression_links(), vec![(lfo, top)]);

        let top2 = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        g.node_mut(top2)
            .params
            .get_mut(TEST_PASS)
            .unwrap()
            .set_expression(&format!("ch('{lfo_name}', 'chan1')"));
        let mut links = g.channel_expression_links();
        links.sort_unstable();
        let mut want = vec![(lfo, top), (lfo, top2)];
        want.sort_unstable();
        assert_eq!(links, want);
    }

    #[test]
    fn families_must_match() {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let top = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        let chop = g.create(root, reg.get("chop_pass").unwrap(), None).unwrap();
        assert!(matches!(
            g.connect(top, chop, 0),
            Err(GraphError::FamilyMismatch(..))
        ));
    }

    #[test]
    fn deleting_a_node_unwires_its_consumers() {
        let (mut g, a, b) = two_chain();
        g.remove(a).unwrap();
        assert!(!g.contains(a));
        assert_eq!(g.node(b).inputs[0], None);
    }

    #[test]
    fn deleting_a_component_takes_its_children() {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let comp = g.create(root, reg.get("container").unwrap(), None).unwrap();
        let inner = g.create(comp, reg.get("pass").unwrap(), None).unwrap();
        assert_eq!(g.path(inner), "/container1/pass1");
        g.remove(comp).unwrap();
        assert!(!g.contains(inner));
    }

    #[test]
    fn mutation_bumps_revision() {
        let (mut g, a, _) = two_chain();
        let before = g.node(a).revision();
        g.set_param(a, TEST_PASS, Value::Float(2.0)).unwrap();
        assert!(g.node(a).revision() > before);
    }

    #[test]
    fn relative_op_path_between_siblings() {
        let (g, a, b) = two_chain();
        assert_eq!(g.relative_op_path(b, a).as_deref(), Some("pass1"));
        assert_eq!(g.relative_op_path(a, b).as_deref(), Some("pass2"));
    }
