use super::*;

    #[test]
    fn merging_makes_implicit_indices_explicit() {
        let a = Geometry {
            points: vec![Point::at(0.0, 0.0, 0.0); 3],
            ..Default::default()
        };
        let b = Geometry {
            points: vec![Point::at(1.0, 0.0, 0.0); 3],
            ..Default::default()
        };
        let m = a.merged(&b);
        assert_eq!(m.num_points(), 6);
        // Without explicit indices the two halves would be one run of six.
        assert_eq!(m.indices, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(m.num_primitives(), 2);
    }

    #[test]
    fn vertex_bytes_are_interleaved_in_the_renderers_layout() {
        let g = Geometry {
            points: vec![Point {
                position: [1.0, 2.0, 3.0],
                normal: [0.0, 1.0, 0.0],
                uv: [0.5, 0.25],
                color: [1.0, 0.0, 0.0, 1.0],
            }],
            ..Default::default()
        };
        let v = g.vertex_bytes();
        assert_eq!(v.len(), 12, "one point is twelve floats");
        assert_eq!(&v[0..3], &[1.0, 2.0, 3.0]);
        assert_eq!(&v[6..8], &[0.5, 0.25]);
        assert_eq!(&v[8..12], &[1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn bounds_of_nothing_are_not_infinite() {
        let g = Geometry::default();
        assert_eq!(g.bounds(), ([0.0; 3], [0.0; 3]));
    }
