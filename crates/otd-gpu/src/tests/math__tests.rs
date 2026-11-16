use super::*;

    fn apply(m: Mat4, v: [f32; 3]) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for (row, cell) in out.iter_mut().enumerate() {
            *cell = m[0][row] * v[0] + m[1][row] * v[1] + m[2][row] * v[2] + m[3][row];
        }
        out
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
    }

    #[test]
    fn identity_leaves_a_point_alone() {
        assert!(close(apply(IDENTITY, [1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]));
    }

    #[test]
    fn trs_applies_scale_then_rotation_then_translation() {
        let m = trs([10.0, 0.0, 0.0], [0.0, 90.0, 0.0], [2.0, 2.0, 2.0]);
        // (1,0,0) scaled to (2,0,0), rotated 90° about Y to (0,0,-2),
        // then translated.
        assert!(
            close(apply(m, [1.0, 0.0, 0.0]), [10.0, 0.0, -2.0]),
            "{:?}",
            apply(m, [1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn a_view_matrix_undoes_the_camera_placement() {
        let (t, r, s) = ([3.0, 4.0, 5.0], [20.0, -35.0, 10.0], [1.0, 1.0, 1.0]);
        let world = trs(t, r, s);
        let view = inverse_trs(t, r, s);
        let round_trip = apply(view, apply(world, [1.0, -2.0, 0.5]));
        assert!(close(round_trip, [1.0, -2.0, 0.5]), "{round_trip:?}");
    }

    #[test]
    fn perspective_puts_the_near_plane_at_zero_and_far_at_one() {
        let p = perspective(60.0, 1.0, 0.1, 100.0);
        let depth_of = |z: f32| {
            let clip_z = p[2][2] * z + p[3][2];
            let clip_w = -z;
            clip_z / clip_w
        };
        assert!(depth_of(-0.1).abs() < 1e-4, "{}", depth_of(-0.1));
        assert!(
            (depth_of(-100.0) - 1.0).abs() < 1e-4,
            "{}",
            depth_of(-100.0)
        );
    }

    #[test]
    fn look_at_puts_the_target_down_the_negative_z_axis() {
        let v = look_at([0.0, 0.0, 5.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let origin_in_view = apply(v, [0.0, 0.0, 0.0]);
        assert!(
            close(origin_in_view, [0.0, 0.0, -5.0]),
            "{origin_in_view:?}"
        );
    }
