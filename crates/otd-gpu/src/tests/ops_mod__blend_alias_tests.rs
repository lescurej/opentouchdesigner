use super::*;
    use crate::ops::composite;

    #[test]
    fn each_named_blend_pins_the_composite_operation_it_is_named_after() {
        let menu = composite::params()["operation"].menu.clone().unwrap();
        let ctx = EvalContext::default();
        let registry = registry();
        let mut graph = otd_core::Graph::new();
        let root = graph.root();

        for (type_name, operation) in [
            ("overTOP", "over"),
            ("underTOP", "under"),
            ("addTOP", "add"),
            ("subtractTOP", "subtract"),
            ("multiplyTOP", "multiply"),
            ("screenTOP", "screen"),
            ("differenceTOP", "difference"),
            ("insideTOP", "inside"),
            ("outsideTOP", "outside"),
            ("crossTOP", "cross"),
        ] {
            let spec = spec(type_name).expect("registered");
            let id = graph
                .create(root, registry.get(type_name).unwrap(), None)
                .unwrap();
            let node = graph.node(id);
            let expected = menu
                .iter()
                .position(|m| m.value == operation)
                .expect("in menu");
            assert_eq!(
                (spec.pack)(node, &ctx)[0][0] as usize,
                expected,
                "{type_name} should composite with `{operation}`"
            );
        }
    }
