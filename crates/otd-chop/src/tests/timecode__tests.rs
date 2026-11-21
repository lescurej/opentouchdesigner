use super::smpte::read_bcd;
use super::*;

fn ltc_samples_for_time(
    t0: f64,
    n: usize,
    rate: f64,
    fps: f64,
    drop_frame: bool,
    level: f32,
) -> Vec<f32> {
    let start = (t0 * rate.max(1.0)).round() as i64;
    ltc_samples_from_index(start, n, rate, fps, drop_frame, level)
}

#[test]
fn ltc_frame_matches_smpte_layout() {
    let smpte = Smpte {
        hour: 12,
        min: 34,
        sec: 56,
        frame: 7,
        frame_frac: 0.0,
        total_frames: 0,
    };
    let bits = encode_ltc_frame(&smpte, false, 30.0);
    assert_eq!(&bits[64..80], &SYNC);
    assert_eq!(bits[48], 0);
    assert_eq!(bits[49], 1);
    assert_eq!(bits[56], 1);
    assert_eq!(bits[57], 0);
    assert_eq!(bits[10], 0);
    assert_eq!(bits.iter().filter(|&&b| b == 0).count() % 2, 0);
    let back = decode_ltc_frame(&bits, 30.0).unwrap();
    assert_eq!((back.hour, back.min, back.sec, back.frame), (12, 34, 56, 7));

    let pal = encode_ltc_frame(&smpte, false, 25.0);
    assert_eq!(pal.iter().filter(|&&b| b == 0).count() % 2, 0);
}

#[test]
fn drop_frame_skips_the_first_two_frames_of_a_minute() {
    let first = smpte_from_drop_frames(0);
    assert_eq!((first.min, first.sec, first.frame), (0, 0, 0));
    let end_min = smpte_from_drop_frames(1799);
    assert_eq!((end_min.min, end_min.sec, end_min.frame), (0, 59, 29));
    let dropped = smpte_from_drop_frames(1800);
    assert_eq!((dropped.min, dropped.sec, dropped.frame), (1, 0, 2));
    let ten = smpte_from_drop_frames(17982);
    assert_eq!((ten.min, ten.sec, ten.frame), (10, 0, 0));
}

#[test]
fn every_frame_starts_on_the_same_polarity() {
    let fps = 30.0f64;
    let rate = 48000.0f64;
    let n = (rate / fps).round() as usize;
    let samples = ltc_samples_for_time(0.0, n + 1, rate, fps, false, 0.5);
    assert_eq!(samples[0], samples[n]);
    assert!(samples[0] * samples[n - 1] < 0.0);
}

#[test]
fn drop_frame_audio_round_trips() {
    let fps = 30000.0f64 / 1001.0;
    let rate = 48000.0f64;
    let n = (rate / fps * 4.0).round() as usize;
    let samples = ltc_samples_for_time(60.06, n, rate, fps, true, 0.5);
    let decoded = decode_ltc_stream(&samples, rate, fps, true).unwrap();
    assert_eq!(decoded.min, 1);
    assert_eq!(decoded.sec, 0);
    assert!((2..8).contains(&decoded.frame), "frame {}", decoded.frame);
}

#[test]
fn ltc_audio_round_trips_at_48k_and_44k() {
    for rate in [48000.0f64, 44100.0] {
        for fps in [24.0f64, 25.0, 30.0] {
            let n = (rate / fps * 3.0).round() as usize;
            let samples = ltc_samples_for_time(12.0, n, rate, fps, false, 0.5);
            let decoded = decode_ltc_stream(&samples, rate, fps, false)
                .unwrap_or_else(|| panic!("no lock at {rate} Hz, {fps} fps"));
            assert_eq!(decoded.sec, 12, "rate {rate} fps {fps}");
            assert!(
                decoded.frame <= 2,
                "frame {} at {rate} {fps}",
                decoded.frame
            );
        }
    }
}

#[test]
fn ltc_decode_ignores_polarity_offset_and_buffer_phase() {
    let fps = 30.0;
    let rate = 48000.0;
    let n = (rate * 2.0) as usize;
    let samples = ltc_samples_for_time(0.0, n, rate, fps, false, 0.5);
    let tail: Vec<f32> = samples[samples.len() - 8192..]
        .iter()
        .map(|s| -s + 0.15)
        .collect();
    assert_ne!(8192 % 10, 0);
    let decoded = decode_ltc_stream(&tail, rate, fps, false).unwrap();
    assert_eq!(decoded.sec, 1);
    assert!(decoded.frame >= 20, "frame {}", decoded.frame);
    assert!(decoded.total_frames > 30);
}

#[test]
fn user_bits_land_in_their_nibbles_and_keep_even_parity() {
    let smpte = Smpte {
        hour: 0,
        min: 0,
        sec: 0,
        frame: 0,
        frame_frac: 0.0,
        total_frames: 0,
    };
    let user = [0x1, 0xa, 0, 0, 0, 0, 0, 0x5];
    let bits = encode_ltc_frame_user(&smpte, false, 30.0, &user);
    assert_eq!(read_bcd(&bits, 4, 4), 1);
    assert_eq!(read_bcd(&bits, 12, 4), 0xa);
    assert_eq!(read_bcd(&bits, 60, 4), 5);
    assert_eq!(bits.iter().filter(|&&b| b == 0).count() % 2, 0);
    let back = decode_ltc_frame(&bits, 30.0).unwrap();
    assert_eq!((back.hour, back.min, back.sec, back.frame), (0, 0, 0, 0));
}

#[test]
fn duplicate_high_fps_repeats_each_frame_number() {
    let stream = LtcStream {
        fps: 60.0,
        drop_frame: false,
        level: 0.5,
        duplicate_high_fps: true,
        user: [0; 8],
    };
    let step = 60.0 / 48000.0;
    let n = 48000;
    let samples = ltc_samples_from_clock(0.0, n, step, false, &stream);
    let decoded = decode_ltc_stream(&samples, 48000.0, 60.0, false).unwrap();
    assert_eq!(decoded.sec, 0);
    assert!(decoded.frame < 30, "frame {}", decoded.frame);
    assert!(decoded.frame >= 20, "frame {}", decoded.frame);
}

#[test]
fn chop_index_from_smpte_channels() {
    let chans = vec![
        ("hour".into(), vec![0.0]),
        ("min".into(), vec![0.0]),
        ("sec".into(), vec![2.0]),
        ("frame".into(), vec![0.0]),
    ];
    assert_eq!(
        chop_timecode_frame_index(&chans, 30.0, "absolute"),
        Some(60)
    );
}

#[test]
fn chop_index_prefers_smpte_over_hour_only_legacy() {
    let chans = vec![
        ("hour".into(), vec![0.0]),
        ("min".into(), vec![0.0]),
        ("sec".into(), vec![1.0]),
        ("frame".into(), vec![15.0]),
    ];
    assert_eq!(
        chop_timecode_frame_index(&chans, 30.0, "absolute"),
        Some(45)
    );
}

#[test]
fn a_timecode_string_parses_with_a_drop_frame_semicolon() {
    let tc = parse_timecode("01:02:03:04", 30.0, false).unwrap();
    assert_eq!((tc.hour, tc.min, tc.sec, tc.frame), (1, 2, 3, 4));
    let drop = parse_timecode("00:01:00;02", 30.0, false).unwrap();
    assert_eq!((drop.min, drop.sec, drop.frame), (1, 0, 2));
    assert_eq!(drop.total_frames, 1800);
}

#[test]
fn sixty_fps_frame_numbers_round_trip() {
    let fps = 60.0f64;
    let rate = 48000.0f64;
    let n = rate as usize;
    let samples = ltc_samples_for_time(0.0, n, rate, fps, false, 0.5);
    let decoded = decode_ltc_stream(&samples, rate, fps, false).unwrap();
    assert_eq!(decoded.sec, 0);
    assert!(decoded.frame >= 45, "frame {}", decoded.frame);
}
