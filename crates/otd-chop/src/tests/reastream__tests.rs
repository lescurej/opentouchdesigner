use super::*;

#[test]
fn stereo_packet_matches_libreastream_layout() {
    let ch0 = [0.42_f32, -0.95, -1.0];
    let ch1 = [0.11_f32, 0.0, -0.5];
    let packets = encode_audio_packets("test", 44100, &[&ch0, &ch1]).unwrap();
    assert_eq!(packets.len(), 1);
    let p = &packets[0];
    assert_eq!(&p[0..4], MAGIC);
    let packet_size = u32::from_le_bytes(p[4..8].try_into().unwrap());
    assert_eq!(packet_size as usize, p.len());
    let mut id = [0u8; 32];
    id[..4].copy_from_slice(b"test");
    assert_eq!(&p[8..40], &id);
    assert_eq!(p[40], 2);
    assert_eq!(u32::from_le_bytes(p[41..45].try_into().unwrap()), 44100);
    let sblocklen = u16::from_le_bytes(p[45..47].try_into().unwrap());
    assert_eq!(sblocklen as usize, 24);
    let mut floats = Vec::new();
    for chunk in p[47..].chunks_exact(4) {
        floats.push(f32::from_le_bytes(chunk.try_into().unwrap()));
    }
    assert_eq!(floats, vec![0.42, -0.95, -1.0, 0.11, 0.0, -0.5]);
}

#[test]
fn many_channels_drain_in_max_size_blocks() {
    let nch = 16usize;
    let mut q = ReaStreamQueue {
        channels: (0..nch).map(|_| VecDeque::new()).collect(),
        finite: true,
        epoch: 0,
        last_block: 0,
    };
    let max_take = MAX_AUDIO_BYTES / (nch * 4);
    for ch in &mut q.channels {
        ch.extend(std::iter::repeat_n(0.25_f32, max_take * 3));
    }
    assert_eq!(q.take_send_block(usize::MAX).unwrap()[0].len(), max_take);
    assert_eq!(q.take_send_block(usize::MAX).unwrap()[0].len(), max_take);
    assert_eq!(q.take_send_block(10).unwrap()[0].len(), 10);
    assert_eq!(q.pending_samples(), max_take - 10);
    assert!(q.take_send_block(0).is_none());
}

#[test]
fn replace_keeps_the_start_of_a_long_clip() {
    let mut q = ReaStreamQueue {
        channels: Vec::new(),
        finite: false,
        epoch: 0,
        last_block: 0,
    };
    let samples: Vec<f32> = (0..QUEUE_CAP_SAMPLES + 10).map(|i| i as f32).collect();
    q.replace_block(&[&samples]);
    assert!(q.finite);
    assert_eq!(q.epoch, 1);
    assert_eq!(q.pending_samples(), QUEUE_CAP_SAMPLES);
    assert_eq!(q.channels[0][0], 0.0);
    q.push_block(&[&[1.0, 2.0]]);
    assert!(!q.finite);
}

#[test]
fn playback_continues_below_the_jitter_watermark() {
    let cushion = 48_000u64 * 200 / 1000;
    let decision = pace(true, 1_000, 0.02, 100, 48_000, cushion, false);
    assert!(matches!(decision, Pace::Send(_)));
}

#[test]
fn stream_waits_for_a_full_jitter_buffer_before_starting() {
    let cushion = 48_000u64 * 200 / 1000;
    assert!(matches!(
        pace(
            false,
            0,
            0.0,
            (cushion - 1) as usize,
            48_000,
            cushion,
            false
        ),
        Pace::Buffer
    ));
    assert!(matches!(
        pace(false, 0, 0.0, cushion as usize, 48_000, cushion, false),
        Pace::Send(_)
    ));
}

#[test]
fn an_empty_stream_waits_without_resetting_the_clock() {
    let cushion = 48_000u64 * 80 / 1000;
    assert!(matches!(
        pace(true, 100, 0.05, 0, 48_000, cushion, false),
        Pace::Wait(_)
    ));
    assert!(matches!(
        pace(true, 100, 0.05, 0, 48_000, cushion, true),
        Pace::Wait(_)
    ));
}

#[test]
fn a_stalled_clock_resyncs_instead_of_bursting_a_second() {
    let cushion = 48_000u64 * 80 / 1000;
    assert!(matches!(
        pace(true, 0, 1.0, 48_000, 48_000, cushion, false),
        Pace::Resync
    ));
}

#[test]
fn a_slow_frame_raises_the_cushion_above_the_setting() {
    let jitter = 48_000u64 * 80 / 1000;
    let frame = 48_000usize * 100 / 1000;
    let cushion = cushion_samples(jitter, frame, 48_000);
    assert!(cushion >= frame as u64 * 2);
    assert!(matches!(
        pace(false, 0, 0.0, frame, 48_000, cushion, false),
        Pace::Buffer
    ));
    assert!(matches!(
        pace(false, 0, 0.0, cushion as usize, 48_000, cushion, false),
        Pace::Send(_)
    ));
}

#[test]
fn caught_up_playback_waits_instead_of_bursting() {
    let cushion = 48_000u64 * 200 / 1000;
    let lead = lead_samples(48_000, cushion);
    let elapsed = 0.05_f64;
    let sent = (elapsed * 48_000.0).floor() as u64 + lead;
    assert!(matches!(
        pace(true, sent, elapsed, 9_000, 48_000, cushion, false),
        Pace::Wait(_)
    ));
}

#[test]
fn large_blocks_split_across_packets() {
    let ch = vec![0.0_f32; 400];
    let packets = encode_audio_packets("default", 48000, &[&ch]).unwrap();
    assert!(packets.len() > 1);
    for p in &packets {
        let sblocklen = u16::from_le_bytes(p[45..47].try_into().unwrap()) as usize;
        assert!(sblocklen <= MAX_AUDIO_BYTES);
    }
}
