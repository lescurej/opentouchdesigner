use super::*;
    use crate::graph::Graph;
    use crate::test_ops::{CountingCooker, TEST_PASS, registry};
    use crate::value::Value;

    /// noise -> level -> null
    fn chain() -> (Graph, Vec<NodeId>) {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let a = g.create(root, reg.get("pass").unwrap(), Some("a")).unwrap();
        let b = g.create(root, reg.get("pass").unwrap(), Some("b")).unwrap();
        let c = g.create(root, reg.get("pass").unwrap(), Some("c")).unwrap();
        g.connect(a, b, 0).unwrap();
        g.connect(b, c, 0).unwrap();
        (g, vec![a, b, c])
    }

    #[test]
    fn a_static_graph_cooks_once_and_then_caches() {
        let (g, n) = chain();
        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        let mut ctx = CookContext::default();

        for _ in 0..10 {
            e.cook_frame(&g, &[n[2]], &ctx, &mut cooker).unwrap();
            ctx.advance_nominal_frame();
        }
        assert_eq!(e.cook_count(n[0]), 1);
        assert_eq!(e.cook_count(n[1]), 1);
        assert_eq!(e.cook_count(n[2]), 1);
    }

    #[test]
    fn editing_a_node_recooks_it_and_everything_downstream_only() {
        let (mut g, n) = chain();
        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        let mut ctx = CookContext::default();
        e.cook_frame(&g, &[n[2]], &ctx, &mut cooker).unwrap();

        ctx.advance_nominal_frame();
        g.set_param(n[1], TEST_PASS, Value::Float(0.5)).unwrap();
        e.cook_frame(&g, &[n[2]], &ctx, &mut cooker).unwrap();

        assert_eq!(e.cook_count(n[0]), 1, "upstream must not re-cook");
        assert_eq!(e.cook_count(n[1]), 2);
        assert_eq!(e.cook_count(n[2]), 2, "downstream must re-cook");
    }

    #[test]
    fn time_dependence_propagates_downstream_but_not_up() {
        let (mut g, n) = chain();
        // Animate the middle node's parameter.
        g.set_expression(n[1], TEST_PASS, "sin(absTime)").unwrap();

        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        let mut ctx = CookContext::default();
        for _ in 0..5 {
            e.cook_frame(&g, &[n[2]], &ctx, &mut cooker).unwrap();
            ctx.advance_nominal_frame();
        }
        assert_eq!(e.cook_count(n[0]), 1, "static upstream stays cached");
        assert_eq!(e.cook_count(n[1]), 5);
        assert_eq!(e.cook_count(n[2]), 5);
        assert!(!e.is_time_dependent(n[0]));
        assert!(e.is_time_dependent(n[1]));
        assert!(e.is_time_dependent(n[2]));
    }

    #[test]
    fn time_sliced_nodes_cook_again_when_the_timeline_frame_does_not_move() {
        let (mut g, n) = chain();
        g.set_expression(n[1], TEST_PASS, "sin(absTime)").unwrap();
        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        // Two repaints inside the same timeline frame. A 120 Hz display does
        // this constantly against a 60 fps project; skipping the second cook
        // drops half of a time-sliced audio stream.
        let ctx = CookContext::default();
        e.cook_frame(&g, &[n[2]], &ctx, &mut cooker).unwrap();
        e.cook_frame(&g, &[n[2]], &ctx, &mut cooker).unwrap();
        assert_eq!(e.cook_count(n[0]), 1, "static upstream stays cached");
        assert_eq!(e.cook_count(n[1]), 2);
        assert_eq!(e.cook_count(n[2]), 2);
    }

    #[test]
    fn a_shared_ancestor_cooks_once_per_frame() {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let src = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        let l = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        let r = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        let join = g.create(root, reg.get("comp2").unwrap(), None).unwrap();
        g.connect(src, l, 0).unwrap();
        g.connect(src, r, 0).unwrap();
        g.connect(l, join, 0).unwrap();
        g.connect(r, join, 1).unwrap();
        // Force every frame to re-cook so the per-frame guard is what's tested.
        g.set_expression(src, TEST_PASS, "absTime").unwrap();

        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        let mut ctx = CookContext::default();
        for _ in 0..4 {
            e.cook_frame(&g, &[join], &ctx, &mut cooker).unwrap();
            ctx.advance_nominal_frame();
        }
        assert_eq!(e.cook_count(src), 4, "once per frame, not twice");
        assert_eq!(e.cook_count(join), 4);
    }

    #[test]
    fn unreached_branches_never_cook() {
        let (mut g, n) = chain();
        let reg = registry();
        let root = g.root();
        let orphan = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        g.connect(n[0], orphan, 0).unwrap();

        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        e.cook_frame(&g, &[n[2]], &CookContext::default(), &mut cooker)
            .unwrap();
        assert_eq!(e.cook_count(orphan), 0);
    }

    #[test]
    fn bypass_passes_through_without_cooking() {
        let (mut g, n) = chain();
        g.node_mut(n[1]).flags.bypass = true;
        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        e.cook_frame(&g, &[n[2]], &CookContext::default(), &mut cooker)
            .unwrap();
        assert_eq!(e.cook_count(n[1]), 0);
        assert_eq!(e.cook_count(n[0]), 1);
        assert_eq!(resolve_bypass(&g, n[1]), Some(n[0]));
    }

    #[test]
    fn a_parameter_named_reference_cooks_and_animates_like_a_wire() {
        let (mut g, n) = chain();
        let reg = registry();
        let root = g.root();
        // `selector` names n[1] by parameter rather than wiring to it.
        let selector = g
            .create(root, reg.get("pass").unwrap(), Some("sel"))
            .unwrap();
        let mut cooker = CountingCooker {
            references: vec![(selector, n[1])],
            ..Default::default()
        };
        g.set_expression(n[0], TEST_PASS, "absTime").unwrap();

        let mut e = CookEngine::new();
        let mut ctx = CookContext::default();
        for _ in 0..3 {
            e.cook_frame(&g, &[selector], &ctx, &mut cooker).unwrap();
            ctx.advance_nominal_frame();
        }
        // Pulling only `selector` must have dragged the referenced branch in.
        assert_eq!(e.cook_count(n[0]), 3);
        assert_eq!(e.cook_count(n[1]), 3);
        assert_eq!(e.cook_count(selector), 3);
        assert!(e.is_time_dependent(selector), "animation must propagate");
    }

    #[test]
    fn a_reference_cycle_degrades_to_a_stale_read_not_a_stack_overflow() {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let a = g.create(root, reg.get("pass").unwrap(), Some("a")).unwrap();
        let b = g.create(root, reg.get("pass").unwrap(), Some("b")).unwrap();
        let mut cooker = CountingCooker {
            references: vec![(a, b), (b, a)],
            ..Default::default()
        };
        let mut e = CookEngine::new();
        e.cook_frame(&g, &[a], &CookContext::default(), &mut cooker)
            .unwrap();
        assert_eq!(e.cook_count(a), 1);
        assert_eq!(e.cook_count(b), 1);
    }

    #[test]
    fn an_intrinsically_animated_source_drives_the_chain() {
        let mut g = Graph::new();
        let reg = registry();
        let root = g.root();
        let mov = g.create(root, reg.get("movie").unwrap(), None).unwrap();
        let out = g.create(root, reg.get("pass").unwrap(), None).unwrap();
        g.connect(mov, out, 0).unwrap();

        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        let mut ctx = CookContext::default();
        for _ in 0..3 {
            e.cook_frame(&g, &[out], &ctx, &mut cooker).unwrap();
            ctx.advance_nominal_frame();
        }
        assert_eq!(e.cook_count(mov), 3);
        assert_eq!(e.cook_count(out), 3);
    }

    #[test]
    fn a_dangling_input_does_not_pull_a_missing_operator() {
        let (mut g, n) = chain();
        let missing = g
            .create(g.root(), registry().get("pass").unwrap(), Some("gone"))
            .unwrap();
        g.remove(missing).unwrap();
        g.node_mut(n[2]).inputs[0] = Some(missing);

        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        e.cook_frame(&g, &[n[2]], &CookContext::default(), &mut cooker)
            .unwrap();
    }

    #[test]
    fn clear_visiting_lets_the_next_cook_frame_run() {
        let (g, n) = chain();
        let mut e = CookEngine::new();
        let mut cooker = CountingCooker::default();
        e.test_set_visiting(n[2], true).unwrap();
        e.clear_visiting();
        e.cook_frame(&g, &[n[2]], &CookContext::default(), &mut cooker)
            .unwrap();
        assert_eq!(e.cook_count(n[2]), 1);
    }
