use std::collections::VecDeque;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::super::decode::Info;
use super::super::Frame;
#[cfg(target_os = "macos")]
use super::super::macos;

pub(crate) const SEEK_AHEAD_LIMIT: i64 = 90;
pub(crate) const QUEUE_DEPTH: usize = 4;
pub(crate) const FIRST_FRAME_GRACE: Duration = Duration::from_secs(5);

// ------------------------------------------------------------------ decoding

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovieDecodeBackend {
    AvFoundation,
    Ffmpeg,
}

impl MovieDecodeBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::AvFoundation => "AVFoundation (cidre)",
            Self::Ffmpeg => "ffmpeg",
        }
    }
}

/// A running decode: a thread, and a short queue of frames it has produced.
pub(crate) struct Reader {
    pub(crate) frames: Receiver<(i64, Arc<Frame>)>,
    pub(crate) stop: Arc<AtomicBool>,
    pub(crate) worker: Option<std::thread::JoinHandle<()>>,
    pub(crate) child: Option<Arc<SharedChild>>,
    pub(crate) file_backend: Option<MovieDecodeBackend>,
    #[cfg(target_os = "macos")]
    pub(crate) capture_hold: Option<macos::MacCaptureHold>,
    #[cfg(target_os = "macos")]
    pub(crate) file_decode_hold: Option<macos::MacFileDecodeHold>,
}

/// A child process that can be killed from the owning thread while the
/// reader thread is blocked on its stdout.
pub(crate) struct SharedChild(std::sync::Mutex<Child>);

impl SharedChild {
    pub(crate) fn new(child: Child) -> Self {
        Self(std::sync::Mutex::new(child))
    }

    fn kill(&self) {
        if let Ok(mut c) = self.0.lock() {
            let _ = c.kill();
        }
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        #[cfg(target_os = "macos")]
        {
            if let Some(h) = &self.file_decode_hold {
                h.cancel();
            }
            if let Some(mut h) = self.capture_hold.take() {
                h.stop();
            }
        }
        if let Some(child) = &self.child {
            child.kill();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// A source being played: what it was opened with, what it is, and the
/// decode currently running.
pub struct Source {
    /// The file path or device index this was opened for, so a changed
    /// parameter reopens rather than quietly keeping the old picture.
    pub key: String,
    pub info: Info,
    pub live: bool,
    pub(crate) reader: Option<Reader>,
    /// The index carried by the newest frame taken off the queue.
    pub(crate) current: i64,
    pub(crate) frame: Option<Arc<Frame>>,
    /// Bumped whenever a different frame is adopted, so the engine uploads
    /// to the GPU only when there is something new to upload.
    pub revision: u64,
    /// A forward seek has been issued and the new stream has not produced a
    /// frame yet.
    ///
    /// Starting an ffmpeg costs about a tenth of a second, and asking for
    /// playback faster than the machine decodes means the playhead has run on
    /// again by the time it is ready — so the next cook seeks again, killing
    /// the process that was about to deliver. Repeat, and the picture sticks
    /// on whatever frame arrived last: at speed 20 it froze ten seconds in.
    ///
    /// Counting cooks was the wrong lever, because `otd render` runs flat out
    /// and twenty-four of its cooks can be less than one process start. What
    /// matters is not how long it has been but whether the last seek has
    /// *landed*, which is this.
    pub(crate) awaiting_seek: bool,
    /// Set by a worker that could not do its job, and read by the cook.
    pub(crate) problem: Arc<std::sync::Mutex<Option<String>>>,
    /// Fractional media frame index for smooth playback (not `floor(time*fps)`).
    pub(crate) playhead: f64,
    /// Last `CookContext::time` seen; detects pause vs advance.
    pub(crate) last_timeline_time: f64,
    /// Last transport `abs_time` applied to the sequential playhead.
    ///
    /// The cook runs once per timeline frame while the app advances
    /// `abs_time` every display frame. The gap is the real interval.
    pub(crate) last_sequential_abs_time: f64,
    /// Previous `wanted` from the cook; a large jump means scrub, not decode lag.
    pub(crate) last_wanted: i64,
    /// Decoded ahead of the playhead while paused or catching up.
    pub(crate) pending_frames: VecDeque<(i64, Arc<Frame>)>,
    /// One future frame removed from the channel when pre-read is full.
    pub(crate) blocked_frame: Option<(i64, Arc<Frame>)>,
    pub(crate) movie_playing: bool,
    /// Specify mode: decode only the requested index, never catch up through intermediates.
    pub(crate) snap_index: bool,
    pub(crate) playback_speed: f64,
    pub(crate) dropped_frames: u64,
    pub(crate) prefer_hw: bool,
    pub(crate) preread: usize,
    pub(crate) max_decode_cpus: u32,
    /// Last mapped frame index from the cook (Info CHOP `index`).
    pub(crate) playback_index: i64,
    pub(crate) playback_length: i64,
    pub(crate) playback_sample_rate: f64,
    pub(crate) playback_ended: bool,
    /// When a live source was opened, and whether it has ever delivered.
    ///
    /// A camera that is refused does not fail: on macOS a denied
    /// `kTCCServiceCamera` leaves AVFoundation with a session that opens
    /// happily, writes nothing to stderr, and never produces a sample. ffmpeg
    /// sits there. So does the node — cooking every frame, reporting nothing,
    /// black. The only evidence available is the absence of a first frame, so
    /// that is what gets timed.
    pub(crate) opened: Option<Instant>,
}
