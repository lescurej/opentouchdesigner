use super::*;
    use otd_sop::{Point, Topology};

    #[test]
    fn sop_preview_describes_one_draw_item() {
        let geo = otd_sop::Geometry {
            points: vec![
                Point::at(-1.0, 0.0, -1.0),
                Point::at(1.0, 0.0, -1.0),
                Point::at(0.0, 0.0, 1.0),
            ],
            indices: vec![0, 1, 2],
            topology: Topology::Triangles,
            ..Default::default()
        };
        let scene = describe_sop_preview(&geo, 1.0, &OrbitCamera::default(), false, false, true);
        assert_eq!(scene.items.len(), 2);
        assert_eq!(scene.items[0].instances.len(), 1);
        assert!(!scene.wireframe);
        let wire = describe_sop_preview(&geo, 1.0, &OrbitCamera::default(), true, false, false);
        assert_eq!(wire.items.len(), 1);
        assert!(wire.wireframe);
    }

    #[test]
    fn fit_distance_scales_with_bounding_sphere() {
        let small = fit_distance(0.5, 1.0, 45.0);
        let large = fit_distance(2.0, 1.0, 45.0);
        assert!(large > small * 3.5);
    }
