use super::*;

    #[test]
    fn a_slice_tracks_real_elapsed_time_not_the_nominal_rate() {
        let mut ctx = CookContext::default();
        assert_eq!(slice_len(CONTROL_RATE, &ctx), 1);
        assert_eq!(slice_len(44100.0, &ctx), 735);

        ctx.dt = 2.0 * ctx.nominal_dt();
        assert_eq!(slice_len(CONTROL_RATE, &ctx), 2);
        assert_eq!(slice_len(44100.0, &ctx), 1470);
    }

    #[test]
    fn a_slice_is_never_empty() {
        let mut ctx = CookContext {
            dt: 0.0,
            ..Default::default()
        };
        assert_eq!(slice_len(60.0, &ctx), 1);
        ctx.dt = 1e9;
        assert_eq!(slice_len(44100.0, &ctx), MAX_SLICE);
    }

    #[test]
    fn the_last_sample_of_a_slice_is_now() {
        let ctx = CookContext {
            time: 10.0,
            dt: 0.1,
            ..Default::default()
        };
        let times: Vec<f64> = slice_times(&ctx, 4).collect();
        assert_eq!(times.len(), 4);
        assert!((times[3] - 10.0).abs() < 1e-12);
        assert!((times[0] - 9.925).abs() < 1e-12);
    }
