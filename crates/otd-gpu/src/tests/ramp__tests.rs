use super::*;

    #[test]
    fn dat_table_parses_three_stops() {
        let rows = vec![
            vec!["pos".into(), "r".into(), "g".into(), "b".into(), "a".into()],
            vec!["0".into(), "0".into(), "0".into(), "0".into(), "1".into()],
            vec!["0.5".into(), "1".into(), "0".into(), "0".into(), "1".into()],
            vec!["1".into(), "1".into(), "1".into(), "1".into(), "1".into()],
        ];
        let keys = keyframes_from_rows(&rows);
        assert_eq!(keys.len(), 3);
        assert!((keys[1].rgba[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn step_holds_left_value_until_next_stop() {
        let keys = vec![
            RampKeyframe {
                pos: 0.0,
                rgba: [0.0, 0.0, 0.0, 1.0],
            },
            RampKeyframe {
                pos: 0.5,
                rgba: [1.0, 0.0, 0.0, 1.0],
            },
            RampKeyframe {
                pos: 1.0,
                rgba: [1.0, 1.0, 1.0, 1.0],
            },
        ];
        let mid = eval_at(&keys, 0.49, RampInterp::Step, 0.0);
        assert!((mid[0] - 0.0).abs() < 1e-5);
        let at = eval_at(&keys, 0.5, RampInterp::Step, 0.0);
        assert!((at[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn baked_lut_middle_matches_linear_stop() {
        let keys = vec![
            RampKeyframe {
                pos: 0.0,
                rgba: [0.0, 0.0, 0.0, 1.0],
            },
            RampKeyframe {
                pos: 1.0,
                rgba: [1.0, 0.0, 0.0, 1.0],
            },
        ];
        let lut = bake_lut(&keys, RampInterp::Linear, 0.0);
        let i = (LUT_WIDTH as usize / 2) * 4;
        assert!(lut[i] > 120 && lut[i] < 135, "mid ramp should be ~0.5 red");
    }
