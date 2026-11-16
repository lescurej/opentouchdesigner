use super::*;

    #[test]
    fn max_time_slice_caps_dt() {
        assert!((clip_frame_dt(0.5, 200.0) - 0.2).abs() < 1e-9);
        assert!((clip_frame_dt(1.0 / 60.0, 200.0) - 1.0 / 60.0).abs() < 1e-9);
    }
