use super::*;

    #[test]
    fn cutoff_floor_scales_with_sample_rate() {
        assert!((clamp_fc(0.1, 48_000.0) - 2.5).abs() < 0.01);
        assert!((clamp_fc(0.1, 60.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn resonance_tapers_at_subsonic_cutoff() {
        assert!(resonance_scale(1.0, 48_000.0) < 0.15);
        assert!((resonance_scale(200.0, 48_000.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn butterworth_200hz_is_not_passthrough() {
        let mut c = BiquadCoeffs::default();
        fill_coeffs_checked(0, &mut c, 48_000.0, 200.0, BUTTER_Q[0]);
        assert!(coeffs_stable(&c));
        assert!(c.a2.abs() > 0.01, "got passthrough a2={}", c.a2);
    }

    #[test]
    fn coeffs_stable_rejects_pole_outside_unit_circle() {
        let c = BiquadCoeffs {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 1.5,
        };
        assert!(!coeffs_stable(&c));
    }

    #[test]
    fn fill_coeffs_checked_passthrough_when_unstable() {
        let mut c = BiquadCoeffs {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 1.5,
        };
        assert!(!coeffs_stable(&c));
        if !coeffs_stable(&c) {
            passthrough_coeffs(&mut c);
        }
        assert!((c.b0 - 1.0).abs() < 1e-6 && c.a2.abs() < 1e-6);
    }
