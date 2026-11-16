
use super::*;

    #[test]
    fn oscillator_block_is_deterministic_across_frames() {
        let rate = 48_000.0;
        let n = 512;
        let mut a = OscillatorState::default();
        let mut b = OscillatorState::default();
        let pitch = [0.0f32];
        let mut out_a = vec![0.0; n];
        let mut out_b = vec![0.0; n];
        for frame in 0..4 {
            a.process_block(
                &mut out_a,
                0,
                440.0,
                1.0,
                0.5,
                0.0,
                0.5,
                false,
                rate,
                &pitch,
                None,
                None,
            );
            b.process_block(
                &mut out_b,
                0,
                440.0,
                1.0,
                0.5,
                0.0,
                0.5,
                false,
                rate,
                &pitch,
                None,
                None,
            );
            assert_eq!(out_a, out_b, "frame {frame}");
            let (p0, prev0, _) = a.store();
            let (p1, prev1, _) = b.store();
            assert_eq!(p0, p1);
            assert_eq!(prev0, prev1);
            a = OscillatorState::load(p0, prev0, true);
            b = OscillatorState::load(p1, prev1, true);
        }
    }
