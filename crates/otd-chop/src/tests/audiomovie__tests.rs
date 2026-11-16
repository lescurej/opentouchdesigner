use super::*;

use super::*;

    fn tone() -> MovieTransport {
        MovieTransport {
            pcm: Arc::new(MoviePcm::new(
                48_000.0,
                1,
                (0..48_000).map(|i| (i as f32).sin()).collect(),
            )),
            channels: 1,
            rate: 48_000.0,
            pos: 0.0,
            speed: 1.0,
            playing: false,
            volume: 1.0,
            last_media: 0.0,
            started: false,
            waiting_on_decode: false,
            fade: 0,
            fade_from: Vec::new(),
            last_out: Vec::new(),
            scratch: Vec::new(),
            device_driven: true,
        }
    }

    #[test]
    fn stereo_reaches_left_and_right_speakers() {
        let mut t = tone();
        t.channels = 2;
        t.pcm = Arc::new(MoviePcm::new(48_000.0, 2, vec![0.25, -0.5]));
        t.sync(0.0, 1.0, false, 1.0);
        let mut buf = [0.0f32; 2];
        t.render(48_000.0, &mut buf, 2);
        assert!((buf[0] - 0.25).abs() < 1e-5, "left {}", buf[0]);
        assert!((buf[1] + 0.5).abs() < 1e-5, "right {}", buf[1]);
    }

    #[test]
    fn five_point_one_folds_into_stereo_and_passes_through_six_speakers() {
        // FL FR FC LFE BL BR
        let src = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let mut stereo = [0.0; 2];
        write_speakers(&src, &mut stereo);
        assert!((stereo[0] - (1.0 + 0.707 + 0.707)).abs() < 1e-3);
        assert!((stereo[1] - 0.707).abs() < 1e-3);

        let mut surround = [0.0; 6];
        write_speakers(&src, &mut surround);
        assert_eq!(surround, src);
    }

    #[test]
    fn steady_playback_does_not_reseek_every_frame() {
        let mut t = tone();
        t.sync(0.0, 1.0, true, 1.0);
        t.playing = true;
        t.device_driven = true;
        let mut buf = [0.0f32; 480];
        t.render(48_000.0, &mut buf, 1);
        let pos = t.pos;
        assert!(
            (pos - 480.0).abs() < 1.0,
            "callback walks the file, pos={pos}"
        );
        // Picture advanced one video frame, not a seek.
        t.sync(0.016, 1.0, true, 1.0);
        assert!(
            (t.pos - pos).abs() < 1.0,
            "a frame step must not snap the read head, pos {}",
            t.pos
        );
        assert!(t.playing);
    }

    #[test]
    fn a_held_playhead_stops_the_read_head_once() {
        let mut t = tone();
        t.sync(1.0, 1.0, true, 1.0);
        let at = t.pos;
        t.sync(1.0, 1.0, true, 1.0);
        t.sync(1.0, 1.0, true, 1.0);
        assert!((t.pos - at).abs() < 1.0);
        assert!(!t.playing, "cue hold must not keep advancing");
    }

    #[test]
    fn a_real_seek_moves_the_read_head() {
        let mut t = tone();
        t.sync(0.0, 1.0, true, 1.0);
        t.playing = true;
        let mut buf = [0.0f32; 480];
        t.render(48_000.0, &mut buf, 1);
        t.sync(0.5, 1.0, true, 1.0);
        assert!(
            (t.pos - 0.5 * 48_000.0).abs() < 2.0,
            "cue jump seeks, pos {}",
            t.pos
        );
    }
