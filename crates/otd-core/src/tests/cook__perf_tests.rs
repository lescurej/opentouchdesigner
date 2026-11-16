use super::*;

use super::*;
    use crate::graph::{Connector, Family, Graph, OpDef, OpRegistry};
    use crate::param::Param;
    use indexmap::IndexMap;

    fn params() -> IndexMap<String, Param> {
        let mut m = IndexMap::new();
        m.insert("gain".into(), Param::float(1.0));
        m
    }

    /// One operator that is time dependent and one that is not, so the two
    /// halves of "what does this cost" can be told apart.
    fn registry() -> OpRegistry {
        let mut r = OpRegistry::new();
        for (type_name, time_dependent) in [("static", false), ("animated", true)] {
            r.register(OpDef {
                type_name,
                label: type_name,
                family: Family::Top,
                inputs: &[],
                input_families: &[],
                summary: "",
                time_dependent,
                params,
                connector: Connector::None,
            });
        }
        r
    }

    struct Nothing;
    impl Cooker for Nothing {
        fn cook(&mut self, _: &Graph, _: NodeId, _: &CookContext) -> Result<CookOutput, CookError> {
            Ok(CookOutput::Updated)
        }
    }

    #[test]
    fn cook_rate_separates_what_runs_every_frame_from_what_does_not() {
        let reg = registry();
        let mut g = Graph::new();
        let root = g.root();
        let still = g.create(root, reg.get("static").unwrap(), None).unwrap();
        let moving = g.create(root, reg.get("animated").unwrap(), None).unwrap();

        let mut engine = CookEngine::new();
        let mut ctx = CookContext::default();
        for _ in 0..20 {
            engine
                .cook_frame(&g, &[still, moving], &ctx, &mut Nothing)
                .unwrap();
            ctx.advance_nominal_frame();
        }

        // The whole point of the demand-driven engine, as a number: a static
        // branch cooks once and then costs nothing, however expensive it is.
        assert!(
            engine.cook_rate(still) < 0.1,
            "static node cooked at {}",
            engine.cook_rate(still)
        );
        assert_eq!(engine.cook_rate(moving), 1.0);
        assert_eq!(engine.cook_count(still), 1);
    }

    #[test]
    fn frame_cost_ranks_by_what_a_frame_actually_pays() {
        let reg = registry();
        let mut g = Graph::new();
        let root = g.root();
        let still = g.create(root, reg.get("static").unwrap(), None).unwrap();
        let moving = g.create(root, reg.get("animated").unwrap(), None).unwrap();

        let mut engine = CookEngine::new();
        let mut ctx = CookContext::default();
        for _ in 0..20 {
            engine
                .cook_frame(&g, &[still, moving], &ctx, &mut Nothing)
                .unwrap();
            ctx.advance_nominal_frame();
        }

        // A node that cooks once contributes almost nothing per frame even if
        // that one cook was slow. Ranking by raw cook time would put it at the
        // top of the list and send someone off optimising the wrong thing.
        assert!(engine.frame_cost_ms(still) <= engine.avg_cook_ms(still) * 0.1);
        assert!((engine.frame_cost_ms(moving) - engine.avg_cook_ms(moving)).abs() < 1e-9);
    }

    #[test]
    fn a_node_that_never_cooked_reports_zero_rather_than_dividing_by_zero() {
        let engine = CookEngine::new();
        let id = NodeId::default();
        assert_eq!(engine.cook_rate(id), 0.0);
        assert_eq!(engine.avg_cook_ms(id), 0.0);
        assert_eq!(engine.frame_cost_ms(id), 0.0);
    }
