use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use super::super::decode::Info;
use super::super::Frame;
use super::reader::{MovieDecodeBackend, Reader, Source, QUEUE_DEPTH};

#[cfg(test)]
pub(crate) fn test_source_with_reader(
    info: Info,
    frames: Receiver<(i64, Arc<Frame>)>,
) -> Source {
    let mut source = test_source(info);
    source.reader = Some(Reader {
        frames,
        stop: Arc::new(AtomicBool::new(false)),
        worker: None,
        child: None,
        file_backend: Some(MovieDecodeBackend::Ffmpeg),
        #[cfg(target_os = "macos")]
        capture_hold: None,
        #[cfg(target_os = "macos")]
        file_decode_hold: None,
    });
    source
}

#[cfg(test)]
pub(crate) fn test_source(info: Info) -> Source {
    let playback_sample_rate = info.fps;
    Source {
        key: "t".into(),
        info,
        live: false,
        reader: None,
        current: -1,
        frame: None,
        revision: 0,
        awaiting_seek: false,
        problem: Default::default(),
        playhead: 0.0,
        last_timeline_time: f64::NAN,
        last_sequential_abs_time: f64::NAN,
        last_wanted: 0,
        pending_frames: VecDeque::new(),
        blocked_frame: None,
        movie_playing: true,
        snap_index: false,
        playback_speed: 1.0,
        dropped_frames: 0,
        prefer_hw: true,
        preread: QUEUE_DEPTH,
        max_decode_cpus: 0,
        playback_index: 0,
        playback_length: 1,
        playback_sample_rate,
        playback_ended: false,
        opened: None,
    }
}
