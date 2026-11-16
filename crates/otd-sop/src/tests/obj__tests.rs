use super::*;

    #[test]
    fn loads_a_simple_triangle() {
        let g = parse("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n", "").unwrap();
        assert_eq!(g.num_points(), 3);
        assert_eq!(g.indices.len(), 3);
    }
