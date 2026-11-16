use std::sync::atomic::AtomicBool;
use std::sync::mpsc::sync_channel;

use super::*;

    #[test]
    fn frame_rates_are_read_as_the_rationals_ffprobe_reports() {
        // 29.97 is `30000/1001`, and getting this wrong drifts a minute of
        // footage by two frames.
        assert!((parse_rational("30000/1001").unwrap() - 29.97).abs() < 0.01);
        assert_eq!(parse_rational("25/1"), Some(25.0));
        assert_eq!(parse_rational("60"), Some(60.0));
        // A still image reports 0/0; it must not become an infinity.
        assert_eq!(parse_rational("0/0"), None);
        assert_eq!(parse_rational("nonsense"), None);
    }

    #[test]
    fn stills_are_decoded_in_process_and_movies_are_not() {
        assert!(is_still("/shots/plate.PNG"));
        assert!(is_still("bg.jpeg"));
        assert!(!is_still("clip.mp4"));
        // A GIF moves, so it goes through ffmpeg like any other video.
        assert!(!is_still("loop.gif"));
        assert!(!is_still("no-extension"));
    }

    /// Exactly what a MacBook Pro camera printed, prefix and all. Keeping
    /// the real strings is the point: this parser exists because there is no
    /// query for a device's modes, only ffmpeg's refusal to use a bad one.
    const REAL_MODE_LIST: &str = "\
[in#0 @ 0x79a810000] Selected video size (1x1) is not supported by the device.
[in#0 @ 0x79a810000] Supported modes:
[in#0 @ 0x79a810000]   640x480@[15.000000 30.000000]fps
[in#0 @ 0x79a810000]   1280x720@[15.000000 30.000000]fps
[in#0 @ 0x79a810000]   1920x1080@[15.000000 30.000000]fps
[in#0 @ 0x79a810000]   1080x1920@[15.000000 30.000000]fps
[in#0 @ 0x79a80c000] Error opening input: Input/output error";

    #[test]
    fn a_devices_modes_are_read_out_of_ffmpegs_refusal() {
        let modes: Vec<Mode> = REAL_MODE_LIST.lines().filter_map(parse_mode_line).collect();
        assert_eq!(modes.len(), 4, "{modes:?}");
        assert_eq!(modes[1].width, 1280);
        assert_eq!(modes[1].height, 720);
        assert_eq!(modes[1].rates, vec![15.0, 30.0]);
        // The surrounding prose must not become a mode.
        assert!(parse_mode_line("[in#0 @ 0x1] Supported modes:").is_none());
        assert!(parse_mode_line("Error opening input: Input/output error").is_none());
    }

    #[test]
    fn the_nearest_real_mode_is_chosen_by_pixel_count() {
        let modes: Vec<Mode> = REAL_MODE_LIST.lines().filter_map(parse_mode_line).collect();

        // Exactly available: taken as asked.
        let (mode, rate) = choose_mode(&modes, 1280, 720, 30.0).unwrap();
        assert_eq!((mode.width, mode.height, rate), (1280, 720, 30.0));

        // Not available: the nearest by area. Asking for 640x1080 does *not*
        // land on 640x480 just because the widths agree — 1280x720 is much
        // closer to the picture that was asked for.
        let (mode, _) = choose_mode(&modes, 640, 1080, 30.0).unwrap();
        assert_eq!((mode.width, mode.height), (1280, 720));

        // And an in-between request rounds to the nearer of its neighbours.
        let (mode, _) = choose_mode(&modes, 1280, 1024, 30.0).unwrap();
        assert_eq!((mode.width, mode.height), (1280, 720));

        // A rate the device has not got becomes the closest one it has,
        // because AVFoundation refuses rather than resamples.
        let (_, rate) = choose_mode(&modes, 640, 480, 60.0).unwrap();
        assert_eq!(rate, 30.0);
        let (_, rate) = choose_mode(&modes, 640, 480, 12.0).unwrap();
        assert_eq!(rate, 15.0);

        // A device that reports nothing — v4l2, dshow — leaves the request
        // alone for ffmpeg to answer for.
        assert!(choose_mode(&[], 1280, 720, 30.0).is_none());
    }

    #[test]
    fn playback_index_advances_with_dt_not_floor_time_jumps() {
        let mut source = crate::video::source::test_source(Info {
                width: 640,
                height: 480,
                fps: 30.0,
                duration: 10.0,
            });
        assert_eq!(
            source.wanted_index_sequential(0.0, 1.0 / 60.0, 1.0, true, false, 30.0),
            0
        );
        assert_eq!(
            source.wanted_index_sequential(1.0 / 60.0, 1.0 / 60.0, 1.0, true, false, 30.0),
            1
        );
        assert_eq!(
            source.wanted_index_sequential(2.0 / 60.0, 1.0 / 60.0, 1.0, true, false, 30.0),
            1
        );
        assert_eq!(source.wanted_index_locked(5.0, 1.0, 30.0, false), 150);
    }

    #[test]
    fn integrated_dt_does_not_run_ahead_of_movie_playing() {
        let mut source = crate::video::source::test_source(Info {
                width: 640,
                height: 480,
                fps: 25.0,
                duration: 10.0,
            });
        source.movie_playing = false;
        source.playback_length = 250;
        for _ in 0..5 {
            source.wanted_index_sequential(0.0, 1.0 / 60.0, 1.0, true, false, 25.0);
        }
        assert!(
            source.playhead.abs() < 1e-6,
            "play param true while movie_playing false must not advance playhead"
        );
        source.set_movie_playing(true);
        source.wanted_index_sequential(0.0, 1.0 / 60.0, 1.0, true, false, 25.0);
        assert!(
            (source.playhead - 25.0 / 60.0).abs() < 0.02,
            "after play starts, one frame should advance by dt*fps, got {}",
            source.playhead
        );
    }

    #[test]
    fn sequential_wall_dt_covers_display_frames_sharing_one_timeline_frame() {
        let mut source = crate::video::source::test_source(Info {
                width: 640,
                height: 480,
                fps: 24.0,
                duration: 10.0,
            });
        source.playback_length = 240;
        let cap = 0.25;
        let d0 = source.sequential_wall_dt(1.0 / 120.0, 1.0 / 120.0, cap);
        source.wanted_index_sequential(0.0, d0, 1.0, true, false, 24.0);
        let d1 = source.sequential_wall_dt(3.0 / 120.0, 1.0 / 120.0, cap);
        assert!(
            (d1 - 2.0 / 120.0).abs() < 1e-9,
            "one cook must cover the skipped display frame, got {d1}"
        );
        source.wanted_index_sequential(0.0, d1, 1.0, true, false, 24.0);
        assert!(
            (source.playhead - 24.0 * 3.0 / 120.0).abs() < 1e-6,
            "24fps media over 3/120s, got {}",
            source.playhead
        );

        let held = source.playhead;
        let paused = source.sequential_wall_dt(1.5, 0.5, cap);
        source.wanted_index_sequential(0.0, paused, 1.0, false, false, 24.0);
        assert!((source.playhead - held).abs() < 1e-9);
        let resumed = source.sequential_wall_dt(1.5 + 1.0 / 60.0, 1.0 / 60.0, cap);
        source.wanted_index_sequential(0.0, resumed, 1.0, true, false, 24.0);
        assert!(
            (source.playhead - held - 24.0 / 60.0).abs() < 1e-6,
            "resume must not catch up the pause, got {}",
            source.playhead
        );
    }

    #[test]
    fn repeat_cue_holds_playhead_until_released() {
        let mut source = crate::video::source::test_source(Info {
                width: 640,
                height: 480,
                fps: 30.0,
                duration: 10.0,
            });
        source.playback_length = 300;
        for frame in 1..=60 {
            let t = frame as f64 / 60.0;
            let dt = source.sequential_wall_dt(t, 1.0 / 60.0, 0.25);
            source.wanted_index_sequential(0.0, dt, 1.0, true, false, 30.0);
        }
        assert!(source.playhead > 20.0);

        source.cue_sequential_to(42, 1.0);
        for frame in 1..=30 {
            let t = 1.0 + frame as f64 / 60.0;
            let dt = source.sequential_wall_dt(t, 1.0 / 60.0, 0.25);
            source.cue_sequential_to(42, t);
            source.wanted_index_sequential(0.0, dt, 1.0, false, false, 30.0);
            assert_eq!(source.playhead, 42.0, "held at cue on frame {frame}");
        }
    }

    #[test]
    fn cue_point_units_map_to_frame_indices() {
        assert_eq!(position_from_units(2.0, "indices", 30.0, 300), 2);
        assert_eq!(position_from_units(2.0, "frames", 30.0, 300), 2);
        assert_eq!(position_from_units(1.0, "seconds", 30.0, 300), 30);
        assert_eq!(position_from_units(0.5, "fraction", 30.0, 101), 50);
    }

    #[test]
    fn sequential_cue_anchors_transport_at_the_cue_index() {
        let mut source = crate::video::source::test_source(Info {
                width: 640,
                height: 480,
                fps: 30.0,
                duration: 10.0,
            });
        source.playback_length = 300;
        for frame in 1..=30 {
            let t = frame as f64 / 60.0;
            let dt = source.sequential_wall_dt(t, 1.0 / 60.0, 0.25);
            source.wanted_index_sequential(0.0, dt, 1.0, true, false, 30.0);
        }
        assert!(source.playhead > 10.0, "should have advanced");

        source.cue_sequential_to(100, 1.0);
        assert_eq!(source.playhead, 100.0);

        let dt = source.sequential_wall_dt(1.0 + 1.0 / 60.0, 1.0 / 60.0, 0.25);
        source.wanted_index_sequential(0.0, dt, 1.0, false, false, 30.0);
        assert_eq!(
            source.playhead, 100.0,
            "repeat-style hold must not advance on the cue frame"
        );

        let dt2 = source.sequential_wall_dt(1.0 + 2.0 / 60.0, 1.0 / 60.0, 0.25);
        source.wanted_index_sequential(0.0, dt2, 1.0, true, false, 30.0);
        assert!(
            (source.playhead - 100.0 - 30.0 / 60.0).abs() < 1e-6,
            "resume advances one frame from the cue, got {}",
            source.playhead
        );
    }

    #[test]
    fn info_index_follows_continuous_playhead_for_chop_slope() {
        let mut source = crate::video::source::test_source(Info {
                width: 640,
                height: 480,
                fps: 25.0,
                duration: 10.0,
            });
        source.playback_length = 250;
        source.wanted_index_sequential(0.0, 1.0 / 60.0, 1.0, true, false, 25.0);
        let index = source
            .info_channels()
            .into_iter()
            .find(|(n, _)| n == "index")
            .map(|(_, v)| v)
            .unwrap();
        assert!(
            (index - 25.0 / 60.0).abs() < 0.02,
            "info index should ramp at media fps, got {index}"
        );
    }

    #[test]
    fn sequential_instances_do_not_snap_to_global_timeline_while_playing() {
        let info = Info {
            width: 640,
            height: 480,
            fps: 30.0,
            duration: 10.0,
        };
        let mut a = crate::video::source::test_source(info.clone());
        let mut b = crate::video::source::test_source(info);
        let dt = 1.0 / 60.0;
        for _ in 0..300 {
            a.wanted_index_sequential(0.0, dt, 1.0, true, false, 30.0);
        }
        let ia = a.wanted_index_sequential(0.0, dt, 1.0, true, false, 30.0);
        let ib = b.wanted_index_sequential(99.0, dt, 1.0, true, false, 30.0);
        assert!(ia > 100, "first instance should have advanced: {ia}");
        assert!(ib < 10, "fresh instance should start near 0: {ib}");
    }

    #[test]
    fn sequential_paused_does_not_snap_to_timeline() {
        let mut source = crate::video::source::test_source(Info {
                width: 640,
                height: 480,
                fps: 30.0,
                duration: 10.0,
            });
        for _ in 0..60 {
            source.wanted_index_sequential(0.0, 1.0 / 60.0, 1.0, true, false, 30.0);
        }
        let at_pause = source.playhead;
        source.wanted_index_sequential(50.0, 0.0, 1.0, false, false, 30.0);
        assert!(
            (source.playhead - at_pause).abs() < 1e-6,
            "paused sequential must not follow timeline scrub"
        );
    }

    #[test]
    fn due_frames_are_drained_even_when_pre_read_is_full() {
        let (tx, frames) = sync_channel(32);
        tx.send((
            19,
            Arc::new(Frame {
                width: 2,
                height: 2,
                pixels: vec![0; 16],
            }),
        ))
        .unwrap();
        for index in 20..24_i64 {
            tx.send((
                index,
                Arc::new(Frame {
                    width: 2,
                    height: 2,
                    pixels: vec![0; 16],
                }),
            ))
            .unwrap();
        }
        drop(tx);
        let mut source = Source {
            key: "t".into(),
            info: Info {
                width: 2,
                height: 2,
                fps: 25.0,
                duration: 20.0,
            },
            live: false,
            reader: Some(Reader {
                frames,
                stop: Arc::new(AtomicBool::new(false)),
                worker: None,
                child: None,
                file_backend: None,
                #[cfg(target_os = "macos")]
                capture_hold: None,
                #[cfg(target_os = "macos")]
                file_decode_hold: None,
            }),
            current: 18,
            frame: Some(Arc::new(Frame {
                width: 2,
                height: 2,
                pixels: vec![1; 16],
            })),
            revision: 1,
            awaiting_seek: false,
            problem: Default::default(),
            playhead: 19.0,
            last_timeline_time: 0.0,
            last_sequential_abs_time: f64::NAN,
            last_wanted: 18,
            pending_frames: (21..25)
                .map(|i| {
                    (
                        i,
                        Arc::new(Frame {
                            width: 2,
                            height: 2,
                            pixels: vec![0; 16],
                        }),
                    )
                })
                .collect(),
            blocked_frame: None,
            movie_playing: true,
            snap_index: false,
            playback_speed: 1.0,
            dropped_frames: 0,
            prefer_hw: true,
            preread: 4,
            max_decode_cpus: 0,
            playback_index: 19,
            playback_length: 500,
            playback_sample_rate: 25.0,
            playback_ended: false,
            opened: None,
        };
        source.advance("", 19);
        assert_eq!(source.current, 19);
    }

    #[test]
    fn a_fast_decoder_stays_on_the_playhead_instead_of_freezing() {
        let (tx, frames) = sync_channel(256);
        for index in 0..200_i64 {
            tx.send((
                index,
                Arc::new(Frame {
                    width: 2,
                    height: 2,
                    pixels: vec![0; 16],
                }),
            ))
            .unwrap();
        }
        let mut source = crate::video::source::test_source_with_reader(
            Info {
                width: 2,
                height: 2,
                fps: 25.0,
                duration: 20.0,
            },
            frames,
        );
        source.playback_length = 500;
        let mut shown = -1_i64;
        for n in 0..240 {
            let wanted =
                source.wanted_index_sequential(n as f64 / 60.0, 1.0 / 60.0, 1.0, true, false, 25.0);
            source.advance("", wanted);
            assert!(
                source.pending_frames.len() <= 4,
                "pre-read window grew to {}",
                source.pending_frames.len()
            );
            if source.current > shown {
                shown = source.current;
            }
            assert!(
                source.current <= wanted,
                "presented {} ahead of playhead {wanted}",
                source.current
            );
        }
        assert!(
            shown >= 90,
            "25fps playback froze at frame {shown} inside a 60Hz cook"
        );
    }

    #[test]
    fn a_still_decodes_to_rgba() {
        // Written and read back here, so the test owns both sides.
        let dir = std::env::temp_dir().join(format!("otd-still-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dot.png");
        let mut buffer = image::RgbaImage::new(2, 1);
        buffer.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        buffer.put_pixel(1, 0, image::Rgba([0, 255, 0, 128]));
        buffer.save(&path).unwrap();

        let frame = decode_still(&path).unwrap();
        assert_eq!((frame.width, frame.height), (2, 1));
        assert_eq!(&frame.pixels[..4], &[255, 0, 0, 255]);
        assert_eq!(&frame.pixels[4..], &[0, 255, 0, 128]);

        assert!(decode_still(&dir.join("nope.png")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
