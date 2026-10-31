//! Video and image input: Movie File In and Video Device In.
//!
//! PLAN.md Phase 1 asks for these on GStreamer. GStreamer is a *system*
//! dependency rather than a crate — it has to be installed, versioned and
//! shipped per platform — and a build that cannot be compiled or verified is
//! not worth writing. §3 of the plan already sanctions the alternative:
//! "ffmpeg subprocess for offline export". This module takes that seriously
//! as the transport for input too.
//!
//! So there are two paths, chosen by what the file is:
//!
//!  * **Still images** — PNG, JPEG, WebP, BMP, TGA, TIFF — are decoded
//!    in-process by the `image` crate. No external tool, no way for it to be
//!    missing, and no subprocess for the case where you just want one picture.
//!  * **Everything that moves** — mp4, mov, mkv, webm, avi, gif — and every
//!    camera runs through an `ffmpeg` subprocess piping raw RGBA on stdout.
//!    If ffmpeg is not installed the node says so, by name, and produces
//!    black rather than failing the cook.
//!
//! There is no one cross-platform “native” movie API: macOS has AVFoundation,
//! Windows has Media Foundation, Linux usually GStreamer. A future in-process
//! backend per OS is possible; today ffmpeg (on PATH or bundled next to the app)
//! is the portable choice that matches what we ship in `package-macos.sh`.
//!
//! What this trades away against GStreamer is in-process zero-copy hardware
//! decode: a pipe costs a memcpy per frame and an upload. What it buys is
//! that it works on all three platforms today, with no build-time dependency
//! and nothing to install to *compile* OpenTouchDesigner.
//!
//! Two rules from the rest of the codebase apply unchanged. Decoding happens
//! on its own thread and hands finished frames to the cook, so nothing here
//! can block a frame. And a missing file, missing camera or missing ffmpeg
//! degrades to a message on the node, because losing a source mid-show must
//! not stop the render.
//!
//! On macOS, movie files are decoded in-process with AVFoundation (cidre);
//! ffmpeg remains the fallback when that path fails and on other platforms.

#[cfg(target_os = "macos")]
mod macos;

use std::collections::VecDeque;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::time::{Duration, Instant};

/// One decoded frame, RGBA8, tightly packed.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// How far ahead it is still cheaper to decode than to seek. Beyond this the
/// reader restarts ffmpeg at the wanted time instead of grinding forward.
const SEEK_AHEAD_LIMIT: i64 = 90;

/// Frames the decoder may run ahead of the cook. Small on purpose: this is
/// the back-pressure that stops a 60 fps source filling memory when the
/// patch is running at 30, and for a camera it is the latency budget.
const QUEUE_DEPTH: usize = 4;

/// Extensions decoded in-process, without ffmpeg.
const STILL_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tga", "tif", "tiff", "ico", "ppm", "pnm",
];

pub fn is_still(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| STILL_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn position_from_units(value: f64, unit: &str, fps: f64, span: i64) -> i64 {
    match unit {
        "seconds" => (value * fps.max(1e-6)).floor() as i64,
        "frames" | "indices" => value.floor() as i64,
        "fraction" => {
            if span <= 1 {
                0
            } else {
                (value.clamp(0.0, 1.0) * (span - 1) as f64).round() as i64
            }
        }
        _ => value.floor() as i64,
    }
}

/// What a source is worth knowing before the first frame arrives.
#[derive(Clone, Debug, Default)]
pub struct Info {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    /// Seconds. Zero for a camera, and for anything ffprobe would not say.
    pub duration: f64,
}

// -------------------------------------------------------- finding the tools
//
// The lookup itself lives in `otd_core::tools`: the movie node, the audio
// file CHOP and the assistant's CLIs all have the same problem with a
// Finder-launched `PATH`, and it was worth solving once.

fn ffmpeg_command() -> Option<Command> {
    otd_core::tools::ffmpeg().map(Command::new)
}

fn ffprobe_command() -> Option<Command> {
    otd_core::tools::ffprobe().map(Command::new)
}

/// Whether both tools were found. Lets a caller say "ffmpeg is missing"
/// rather than guessing at why a file would not open.
pub fn tools_installed() -> bool {
    otd_core::tools::media_tools_installed()
}

/// One wording for the one thing the user has to do about it.
pub fn missing_ffmpeg() -> String {
    otd_core::tools::missing_ffmpeg()
}

// ------------------------------------------------------------------ probing

/// Ask ffprobe what a file is. Returns `None` when ffprobe is missing or the
/// file is not media — the caller turns that into a message on the node.
pub fn probe(path: &str) -> Option<Info> {
    #[cfg(target_os = "macos")]
    if let Some(info) = macos::probe(path) {
        return Some(info);
    }
    let out = ffprobe_command()?
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate:format=duration",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let stream = json.get("streams")?.get(0)?;
    let width = stream.get("width")?.as_u64()? as u32;
    let height = stream.get("height")?.as_u64()? as u32;
    let fps = stream
        .get("r_frame_rate")
        .and_then(|v| v.as_str())
        .and_then(parse_rational)
        .filter(|f| *f > 0.0)
        .unwrap_or(30.0);
    let duration = json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);
    Some(Info {
        width,
        height,
        fps,
        duration,
    })
}

/// ffprobe reports frame rates as `30000/1001`, not as a decimal.
fn parse_rational(text: &str) -> Option<f64> {
    match text.split_once('/') {
        Some((n, d)) => {
            let (n, d) = (n.trim().parse::<f64>().ok()?, d.trim().parse::<f64>().ok()?);
            (d != 0.0).then_some(n / d)
        }
        None => text.trim().parse().ok(),
    }
}

/// The capture input ffmpeg uses for a camera on this platform.
pub fn camera_input() -> (&'static str, &'static str) {
    if cfg!(target_os = "macos") {
        ("avfoundation", "0")
    } else if cfg!(target_os = "windows") {
        ("dshow", "video=default")
    } else {
        ("v4l2", "/dev/video0")
    }
}

/// One thing a camera can actually do.
#[derive(Clone, Debug, PartialEq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub rates: Vec<f64>,
}

/// A line of ffmpeg's supported-mode list: `  1280x720@[15.000000 30.000000]fps`,
/// usually behind a `[in#0 @ 0x7f…]` prefix.
fn parse_mode_line(line: &str) -> Option<Mode> {
    // Drop any bracketed logging prefix without disturbing the `@[...]` that
    // carries the rates.
    let body = match line.rfind("] ") {
        Some(i) if line.starts_with('[') => &line[i + 2..],
        _ => line,
    };
    let (size, rest) = body.trim().split_once('@')?;
    let (w, h) = size.trim().split_once('x')?;
    let width = w.trim().parse().ok()?;
    let height = h.trim().parse().ok()?;
    let rates: Vec<f64> = rest
        .trim_start_matches('[')
        .split(']')
        .next()?
        .split_whitespace()
        .filter_map(|r| r.parse().ok())
        .collect();
    (width > 0 && height > 0).then_some(Mode {
        width,
        height,
        rates,
    })
}

/// Ask a capture device what it can do.
///
/// There is no query for this, so the question is asked by requesting
/// something impossible: ffmpeg refuses and prints the list. It costs a third
/// of a second and no camera warm-up, which is far cheaper than opening the
/// device several times to find out by trial.
pub fn camera_modes(format: &str, device: &str) -> Vec<Mode> {
    let Some(mut ffmpeg) = ffmpeg_command() else {
        return Vec::new();
    };
    let out = ffmpeg
        .args(["-hide_banner", "-v", "error", "-f", format])
        .args(["-video_size", "1x1", "-framerate", "1"])
        .args(["-i", device, "-frames:v", "1", "-f", "null", "-"])
        .stdin(Stdio::null())
        .output();
    let Ok(out) = out else { return Vec::new() };
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(parse_mode_line)
        .collect()
}

/// The mode closest to what the patch asked for, and a rate the device has.
///
/// "Closest" is by pixel count rather than by width: asking for 1280×720 on a
/// portrait camera should not land on 1080×1920 just because a width matches.
fn choose_mode(modes: &[Mode], want_w: u32, want_h: u32, want_fps: f64) -> Option<(Mode, f64)> {
    let want_pixels = (want_w as i64) * (want_h as i64);
    let best = modes.iter().min_by_key(|m| {
        let pixels = (m.width as i64) * (m.height as i64);
        (pixels - want_pixels).abs()
    })?;
    let rate = best
        .rates
        .iter()
        .copied()
        .min_by(|a, b| {
            (a - want_fps)
                .abs()
                .partial_cmp(&(b - want_fps).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(want_fps);
    Some((best.clone(), rate))
}

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
struct Reader {
    frames: Receiver<(i64, Arc<Frame>)>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    /// Kept so the pipe can be killed rather than waited on.
    child: Option<Arc<SharedChild>>,
    file_backend: Option<MovieDecodeBackend>,
    #[cfg(target_os = "macos")]
    capture_hold: Option<macos::MacCaptureHold>,
    #[cfg(target_os = "macos")]
    file_decode_hold: Option<macos::MacFileDecodeHold>,
}

/// A child process that can be killed from the owning thread while the
/// reader thread is blocked on its stdout.
struct SharedChild(std::sync::Mutex<Child>);

impl SharedChild {
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
    reader: Option<Reader>,
    /// The index carried by the newest frame taken off the queue.
    current: i64,
    frame: Option<Arc<Frame>>,
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
    awaiting_seek: bool,
    /// Set by a worker that could not do its job, and read by the cook.
    problem: Arc<std::sync::Mutex<Option<String>>>,
    /// Fractional media frame index for smooth playback (not `floor(time*fps)`).
    playhead: f64,
    /// Last `CookContext::time` seen; detects pause vs advance.
    last_timeline_time: f64,
    /// Last transport `abs_time` applied to the sequential playhead.
    ///
    /// The cook runs once per timeline frame while the app advances
    /// `abs_time` every display frame. The gap is the real interval.
    last_sequential_abs_time: f64,
    /// Previous `wanted` from the cook; a large jump means scrub, not decode lag.
    last_wanted: i64,
    /// Decoded ahead of the playhead while paused or catching up.
    pending_frames: VecDeque<(i64, Arc<Frame>)>,
    /// One future frame removed from the channel when pre-read is full.
    blocked_frame: Option<(i64, Arc<Frame>)>,
    movie_playing: bool,
    /// Specify mode: decode only the requested index, never catch up through intermediates.
    snap_index: bool,
    pub(crate) playback_speed: f64,
    dropped_frames: u64,
    prefer_hw: bool,
    preread: usize,
    max_decode_cpus: u32,
    /// Last mapped frame index from the cook (Info CHOP `index`).
    playback_index: i64,
    playback_length: i64,
    playback_sample_rate: f64,
    playback_ended: bool,
    /// When a live source was opened, and whether it has ever delivered.
    ///
    /// A camera that is refused does not fail: on macOS a denied
    /// `kTCCServiceCamera` leaves AVFoundation with a session that opens
    /// happily, writes nothing to stderr, and never produces a sample. ffmpeg
    /// sits there. So does the node — cooking every frame, reporting nothing,
    /// black. The only evidence available is the absence of a first frame, so
    /// that is what gets timed.
    opened: Option<Instant>,
}

/// How long a camera gets to produce its first frame before the node says so.
///
/// Generous on purpose: a capture device negotiating a mode can take a couple
/// of seconds, and crying wolf on a camera that was merely slow is worse than
/// waiting. Nothing after this is a device that is not going to answer.
const FIRST_FRAME_GRACE: Duration = Duration::from_secs(5);

impl Source {
    /// Open a still image: one frame, decoded once, on its own thread.
    pub fn still(key: String, path: &Path) -> Source {
        let (tx, frames) = sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        // The decode happens off the cook thread, so its failure has to
        // travel back rather than being thrown away — an unreadable image
        // that reports nothing is indistinguishable from one still loading.
        let problem: Arc<std::sync::Mutex<Option<String>>> = Arc::new(std::sync::Mutex::new(None));
        let sink = problem.clone();
        let path = path.to_path_buf();
        let _ = std::thread::Builder::new()
            .name("otd-image-decode".into())
            .spawn(move || match decode_still(&path) {
                Ok(frame) => {
                    let _ = tx.send((0, Arc::new(frame)));
                }
                Err(e) => {
                    if let Ok(mut slot) = sink.lock() {
                        *slot = Some(e);
                    }
                }
            });
        Source {
            key,
            // Filled in from the frame itself when it arrives — an image
            // needs no probe.
            info: Info::default(),
            live: false,
            reader: Some(Reader {
                frames,
                stop,
                worker: None,
                child: None,
                file_backend: None,
                #[cfg(target_os = "macos")]
                capture_hold: None,
                #[cfg(target_os = "macos")]
                file_decode_hold: None,
            }),
            current: -1,
            frame: None,
            revision: 0,
            awaiting_seek: false,
            problem,
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
            playback_sample_rate: 1.0,
            playback_ended: false,
            opened: None,
        }
    }

    pub fn set_decode_tuning(&mut self, prefer_hw: bool, preread: usize, max_cpus: u32) {
        self.prefer_hw = prefer_hw;
        self.preread = preread.clamp(1, 120);
        self.max_decode_cpus = max_cpus;
    }

    /// Open a media file through ffmpeg, starting at `start_index`.
    pub fn file(key: String, path: &str, info: Info, start_index: i64) -> Result<Source, String> {
        let sample_rate = info.fps.max(1e-6);
        let mut source = Source {
            key,
            info,
            live: false,
            reader: None,
            current: -1,
            frame: None,
            revision: 0,
            awaiting_seek: false,
            problem: Default::default(),
            playhead: start_index as f64,
            last_timeline_time: f64::NAN,
            last_sequential_abs_time: f64::NAN,
            last_wanted: start_index,
            pending_frames: VecDeque::new(),
            blocked_frame: None,
            movie_playing: true,
            snap_index: false,
            playback_speed: 1.0,
            dropped_frames: 0,
            prefer_hw: true,
            preread: QUEUE_DEPTH,
            max_decode_cpus: 0,
            playback_index: start_index,
            playback_length: 1,
            playback_sample_rate: sample_rate,
            playback_ended: false,
            opened: None,
        };
        source.reader = Some(source.spawn_file(path, start_index)?);
        source.current = start_index - 1;
        Ok(source)
    }

    /// Open a camera. Always live: there is no seeking a present moment.
    ///
    /// A capture device only does the modes it does — AVFoundation refuses a
    /// size, rate or pixel format it has not got, rather than picking
    /// something near — so the requested resolution is negotiated down to a
    /// real one rather than passed through and left to fail.
    pub fn camera(
        key: String,
        device: &str,
        width: u32,
        height: u32,
        fps: f64,
    ) -> Result<Source, String> {
        let (format, default_device) = camera_input();
        let device = if device.trim().is_empty() {
            default_device.to_string()
        } else {
            device.trim().to_string()
        };

        #[cfg(target_os = "macos")]
        {
            let problem: Arc<std::sync::Mutex<Option<String>>> = Default::default();
            if let Ok((reader, info)) = macos::spawn_camera_reader(
                &device,
                width,
                height,
                fps,
                QUEUE_DEPTH,
                problem.clone(),
            ) {
                let sample_rate = info.fps.max(1e-6);
                return Ok(Source {
                    key,
                    info,
                    live: true,
                    reader: Some(reader),
                    current: -1,
                    frame: None,
                    revision: 0,
                    awaiting_seek: false,
                    problem,
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
                    playback_sample_rate: sample_rate,
                    playback_ended: false,
                    opened: Some(Instant::now()),
                });
            }
        }

        let modes = camera_modes(format, &device);
        let (chosen_w, chosen_h, chosen_fps) = match choose_mode(&modes, width, height, fps) {
            Some((mode, rate)) => (mode.width, mode.height, rate),
            // Nothing reported: v4l2 and dshow do not print a mode list, so
            // pass the request through and let ffmpeg answer for it.
            None => (width, height, fps),
        };

        let mut command = ffmpeg_command().ok_or_else(missing_ffmpeg)?;
        command.args(["-hide_banner", "-v", "error", "-f", format]);
        // AVFoundation's default of yuv420p is not one of the formats a Mac
        // camera offers; uyvy422 is what they all do.
        if format == "avfoundation" {
            command.args(["-pix_fmt", "uyvy422"]);
        }
        command
            .args(["-framerate", &format!("{chosen_fps}")])
            .args(["-video_size", &format!("{chosen_w}x{chosen_h}")])
            .args(["-i", &device]);
        let problem: Arc<std::sync::Mutex<Option<String>>> = Default::default();
        let reader = spawn_ffmpeg(command, chosen_w, chosen_h, 0, problem.clone(), QUEUE_DEPTH)?;
        Ok(Source {
            key,
            info: Info {
                width: chosen_w,
                height: chosen_h,
                fps: chosen_fps,
                duration: 0.0,
            },
            live: true,
            reader: Some(reader),
            current: -1,
            frame: None,
            revision: 0,
            awaiting_seek: false,
            problem,
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
            playback_sample_rate: chosen_fps.max(1e-6),
            playback_ended: false,
            opened: Some(Instant::now()),
        })
    }

    pub fn set_playback_report(&mut self, index: i64, length: i64, sample_rate: f64, ended: bool) {
        self.playback_index = index;
        self.playback_length = length.max(1);
        self.playback_sample_rate = sample_rate.max(1e-6);
        self.playback_ended = ended;
    }

    pub fn info_channels(&self) -> Vec<(String, f32)> {
        let mut out = Vec::new();
        let w = self
            .info
            .width
            .max(self.frame.as_ref().map(|f| f.width).unwrap_or(0));
        let h = self
            .info
            .height
            .max(self.frame.as_ref().map(|f| f.height).unwrap_or(0));
        let length = self.playback_length.max(1);
        let index = self.playhead as f32;
        let true_index = self.current.max(0);
        let rate = self.playback_sample_rate;
        out.push(("index".into(), index));
        out.push(("true_index".into(), true_index as f32));
        out.push(("length".into(), length as f32));
        out.push(("fraction".into(), index / length as f32));
        out.push(("sample_rate".into(), rate as f32));
        out.push(("duration".into(), self.info.duration as f32));
        out.push(("resx".into(), w as f32));
        out.push(("resy".into(), h as f32));
        out.push(("open".into(), 1.0));
        out.push((
            "reader".into(),
            if self.reader.is_some() { 1.0 } else { 0.0 },
        ));
        out.push(("pending".into(), self.pending_frames.len() as f32));
        out.push(("pre_read_misses".into(), self.dropped_frames as f32));
        out.push((
            "awaiting_seek".into(),
            if self.awaiting_seek { 1.0 } else { 0.0 },
        ));
        out.push(("playhead".into(), self.playhead as f32));
        out.push(("ended".into(), if self.playback_ended { 1.0 } else { 0.0 }));
        let decode_backend = self
            .movie_decode_backend()
            .map(|be| match be {
                MovieDecodeBackend::AvFoundation => 0.0,
                MovieDecodeBackend::Ffmpeg => 1.0,
            })
            .unwrap_or(-1.0);
        out.push(("decode_backend".into(), decode_backend));
        if self.live {
            out.push(("live".into(), 1.0));
        }
        out
    }

    pub fn set_movie_playing(&mut self, playing: bool) {
        if self.live || playing == self.movie_playing {
            return;
        }
        self.movie_playing = playing;
    }

    pub fn set_snap_index(&mut self, snap: bool) {
        self.snap_index = snap;
    }

    pub fn movie_playback_status(&self, ideal: i64) -> Option<String> {
        if self.live {
            return None;
        }
        let lag = ideal.saturating_sub(self.current.max(0));
        let decode = if self.reader.is_some() { "on" } else { "off" };
        if lag == 0 && self.dropped_frames == 0 && self.reader.is_some() {
            return None;
        }
        Some(format!(
            "lag {lag} · pending {} · decode {decode} · drops {}",
            self.pending_frames.len(),
            self.dropped_frames
        ))
    }

    fn best_pending_upto(&mut self, wanted: i64) -> Option<(i64, Arc<Frame>)> {
        let mut best: Option<(i64, Arc<Frame>)> = None;
        while let Some((idx, _)) = self.pending_frames.front() {
            if *idx > wanted {
                break;
            }
            let (idx, frame) = self.pending_frames.pop_front().unwrap();
            if best.as_ref().is_none_or(|(b, _)| idx >= *b) {
                best = Some((idx, frame));
            }
        }
        best
    }

    fn pending_limit(&self) -> usize {
        self.preread.clamp(1, 120)
    }

    fn push_pending(&mut self, index: i64, frame: Arc<Frame>) -> bool {
        if index <= self.current.max(0) || self.pending_frames.len() >= self.pending_limit() {
            return false;
        }
        self.pending_frames.push_back((index, frame));
        true
    }

    fn restart_reader(&mut self, path: &str, at: i64) {
        self.pending_frames.clear();
        self.blocked_frame = None;
        if let Some(old) = self.reader.take() {
            drop(old);
        }
        if let Ok(reader) = self.spawn_file(path, at.max(0)) {
            self.reader = Some(reader);
            if !self.snap_index {
                self.current = at.max(0) - 1;
            }
            self.awaiting_seek = true;
        }
    }

    fn needs_seek(&self, wanted: i64, prev_wanted: i64) -> bool {
        wanted < prev_wanted || wanted - prev_wanted > SEEK_AHEAD_LIMIT
    }

    /// Media frame index to decode, from timeline time and playback speed.
    ///
    /// Advances by `dt * speed * fps` while the timeline runs so a dropped
    /// UI frame does not jump two indices at once; snaps on scrub or pause.
    pub fn wanted_index(&mut self, timeline_time: f64, dt: f64, speed: f64) -> i64 {
        self.wanted_index_sequential(timeline_time, dt, speed, true, false, self.info.fps)
    }

    pub fn wanted_index_locked(
        &mut self,
        timeline_time: f64,
        speed: f64,
        fps: f64,
        interp: bool,
    ) -> i64 {
        let fps = fps.max(1e-6);
        if self.info.fps <= 0.0 || self.live {
            return 0;
        }
        self.playhead = (timeline_time * speed * fps).max(0.0);
        self.last_timeline_time = timeline_time;
        if interp {
            self.playhead.round() as i64
        } else {
            self.playhead.floor() as i64
        }
    }

    /// Sequential play (TouchDesigner): advances at `dt × speed × sample_rate`
    /// and ignores timeline position while playing.
    pub fn wanted_index_sequential(
        &mut self,
        timeline_time: f64,
        dt: f64,
        speed: f64,
        playing: bool,
        interp: bool,
        fps: f64,
    ) -> i64 {
        let fps = fps.max(1e-6);
        if self.info.fps <= 0.0 || self.live {
            return 0;
        }
        let advancing = playing && speed != 0.0 && self.movie_playing;
        if advancing && dt > 0.0 {
            self.playhead += dt * speed * fps;
            if self.playhead < 0.0 {
                self.playhead = 0.0;
            }
        }
        self.last_timeline_time = timeline_time;
        if interp {
            self.playhead.round() as i64
        } else {
            self.playhead.floor() as i64
        }
    }

    pub fn seek_playhead_to(&mut self, index: i64) {
        self.playhead = index as f64;
    }

    pub fn reset_sequential_playback(&mut self) {
        self.playhead = 0.0;
        self.last_timeline_time = f64::NAN;
        self.last_sequential_abs_time = f64::NAN;
    }

    /// Seconds of transport time since the previous sequential cook.
    ///
    /// The first call uses `fallback_dt` (this frame's `ctx.dt`). Later calls
    /// use the `abs_time` gap, so several display frames collapsed into one
    /// timeline frame still move the playhead. `cap` drops a hitch instead of
    /// seeking across it. Pause does not move `abs_time`, so resume continues
    /// at the same index.
    pub fn sequential_wall_dt(&mut self, abs_time: f64, fallback_dt: f64, cap: f64) -> f64 {
        let cap = cap.max(0.0);
        let dt = if self.last_sequential_abs_time.is_nan() {
            fallback_dt.max(0.0).min(cap)
        } else {
            (abs_time - self.last_sequential_abs_time).max(0.0).min(cap)
        };
        self.last_sequential_abs_time = abs_time;
        dt
    }

    fn spawn_file(&self, path: &str, start_index: i64) -> Result<Reader, String> {
        #[cfg(target_os = "macos")]
        if self.prefer_hw {
            if let Ok(reader) = macos::spawn_file_reader(
                path,
                &self.info,
                start_index,
                self.preread,
                self.problem.clone(),
            ) {
                return Ok(reader);
            }
        }
        let mut command = ffmpeg_command().ok_or_else(missing_ffmpeg)?;
        command.args(["-v", "error"]);
        if self.max_decode_cpus > 0 {
            command.args(["-threads", &self.max_decode_cpus.to_string()]);
        } else {
            command.args(["-threads", "0"]);
        }
        if start_index > 0 {
            // Before -i, so ffmpeg seeks the container rather than decoding
            // and discarding everything up to the seek point.
            command.args(["-ss", &format!("{}", start_index as f64 / self.info.fps)]);
        }
        command
            .args(["-i", path])
            // A fixed output rate makes frame index and time the same fact,
            // which is what lets a seek be computed rather than searched for.
            .args(["-r", &format!("{}", self.info.fps)]);
        spawn_ffmpeg(
            command,
            self.info.width,
            self.info.height,
            start_index,
            self.problem.clone(),
            self.preread,
        )
    }

    fn route_incoming_frame(
        &mut self,
        wanted: i64,
        index: i64,
        frame: Arc<Frame>,
        pick: &mut Option<(i64, Arc<Frame>)>,
    ) -> bool {
        if self.snap_index {
            if index != wanted {
                return true;
            }
            *pick = Some((index, frame));
            return true;
        }
        if index <= wanted {
            if pick.as_ref().is_none_or(|(i, _)| index >= *i) {
                *pick = Some((index, frame));
            }
            return true;
        }
        if self.push_pending(index, frame) {
            return true;
        }
        false
    }

    fn drain_reader(&mut self, wanted: i64, pick: &mut Option<(i64, Arc<Frame>)>) {
        loop {
            if let Some((index, frame)) = self.blocked_frame.take() {
                if !self.route_incoming_frame(wanted, index, frame.clone(), pick) {
                    self.blocked_frame = Some((index, frame));
                    return;
                }
                continue;
            }
            let Some(reader) = &self.reader else {
                return;
            };
            match reader.frames.try_recv() {
                Ok((index, frame)) => {
                    if !self.route_incoming_frame(wanted, index, frame.clone(), pick) {
                        self.blocked_frame = Some((index, frame));
                        return;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.reader = None;
                    return;
                }
            }
        }
    }

    pub fn advance(&mut self, path: &str, wanted: i64) -> Option<&Arc<Frame>> {
        let wanted = wanted.max(0);
        let prev_wanted = self.last_wanted;
        self.last_wanted = wanted;

        if self.live {
            while let Some(reader) = &self.reader {
                match reader.frames.try_recv() {
                    Ok((index, frame)) => {
                        self.current = index;
                        self.frame = Some(frame);
                        self.awaiting_seek = false;
                        self.opened = None;
                        self.revision += 1;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.reader = None;
                        break;
                    }
                }
            }
            return self.frame.as_ref();
        }

        if self.info.fps > 0.0 {
            if self.snap_index {
                if wanted != prev_wanted {
                    self.restart_reader(path, wanted);
                }
            } else if self.needs_seek(wanted, prev_wanted) {
                self.restart_reader(path, wanted);
            }
        }
        if !self.snap_index {
            if self.info.fps > 0.0 && self.reader.is_none() && self.frame.is_none() {
                self.restart_reader(path, wanted);
            } else if self.reader.is_none() && self.movie_playing && wanted > self.current.max(0) {
                self.restart_reader(path, self.current.max(0));
            }
        } else if self.info.fps > 0.0 && self.reader.is_none() && self.frame.is_none() {
            self.restart_reader(path, wanted);
        }

        let mut pick = if self.snap_index {
            None
        } else {
            self.best_pending_upto(wanted)
        };
        if self.snap_index {
            pick = self
                .pending_frames
                .iter()
                .find(|(i, _)| *i == wanted)
                .map(|(i, f)| (*i, f.clone()));
            if pick.is_none() && self.current == wanted {
                if let Some(f) = self.frame.clone() {
                    pick = Some((self.current, f));
                }
            }
        } else if pick.is_none() && self.current >= 0 && self.current <= wanted {
            if let Some(f) = self.frame.clone() {
                pick = Some((self.current, f));
            }
        }

        self.drain_reader(wanted, &mut pick);

        if let Some((index, frame)) = pick {
            if self.current != index || !self.frame.as_ref().is_some_and(|f| Arc::ptr_eq(f, &frame))
            {
                if self.movie_playing && self.current >= 0 && index > self.current + 1 {
                    self.dropped_frames = self
                        .dropped_frames
                        .saturating_add((index - self.current - 1) as u64);
                }
                self.current = index;
                self.frame = Some(frame);
                self.awaiting_seek = false;
                self.opened = None;
                self.revision += 1;
                if self.info.width == 0 {
                    self.info.width = self.frame.as_ref().unwrap().width;
                    self.info.height = self.frame.as_ref().unwrap().height;
                }
            }
        }

        self.frame.as_ref()
    }

    /// Whether the decode has finished and nothing more will arrive.
    pub fn ended(&self) -> bool {
        self.reader.is_none()
    }

    pub fn frame(&self) -> Option<&Arc<Frame>> {
        self.frame.as_ref()
    }

    /// What went wrong on the decode thread, if anything did.
    ///
    /// ffmpeg's own words first — when it has any, they are better than
    /// anything guessed from the outside. A camera that was refused has none:
    /// see [`silent_camera`].
    pub fn problem(&self) -> Option<String> {
        let said = self.problem.lock().ok().and_then(|p| p.clone());
        said.or_else(|| self.silent_camera())
    }

    pub fn movie_decode_backend(&self) -> Option<MovieDecodeBackend> {
        if self.live {
            return None;
        }
        self.reader.as_ref()?.file_backend
    }

    /// A live source that has produced nothing for too long.
    ///
    /// This exists because the failure it names is otherwise invisible. On
    /// macOS a denied camera is not an error anywhere: ffmpeg starts, opens
    /// the AVFoundation session, writes nothing to stderr and never exits,
    /// and the node cooks a black texture every frame for as long as you care
    /// to watch it. Somebody looking at that has no way to learn that the
    /// answer is one checkbox in System Settings.
    ///
    /// Worded as a suspicion rather than a diagnosis. Access is only the
    /// likeliest cause — a camera held by another app, or one that has been
    /// unplugged since the mode probe, look identical from here — so it says
    /// what it observed first and what to try second.
    fn silent_camera(&self) -> Option<String> {
        let opened = self.opened?;
        if !self.live || opened.elapsed() < FIRST_FRAME_GRACE {
            return None;
        }
        Some(if cfg!(target_os = "macos") {
            "the camera has not produced a frame — check System Settings ▸ \
             Privacy & Security ▸ Camera, and that nothing else is using it"
                .to_string()
        } else {
            "the camera has not produced a frame — check that it is connected \
             and that nothing else is using it"
                .to_string()
        })
    }
}

/// Start ffmpeg writing raw RGBA to stdout and a thread reading frames off it.
fn spawn_ffmpeg(
    mut command: Command,
    width: u32,
    height: u32,
    start_index: i64,
    problem: Arc<std::sync::Mutex<Option<String>>>,
    queue_depth: usize,
) -> Result<Reader, String> {
    if width == 0 || height == 0 {
        return Err("source has no picture size".into());
    }
    let mut child = command
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"])
        .stdout(Stdio::piped())
        // Kept rather than discarded: when a capture device refuses a mode,
        // ffmpeg's own words list the ones it would accept, and that belongs
        // on the node where somebody can read it.
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "ffmpeg is not installed — `brew install ffmpeg`, or use a PNG/JPEG still".into()
            }
            _ => format!("could not start ffmpeg: {e}"),
        })?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or("ffmpeg produced no output pipe")?;
    if let Some(mut stderr) = child.stderr.take() {
        let sink = problem.clone();
        let _ = std::thread::Builder::new()
            .name("otd-video-stderr".into())
            .spawn(move || {
                let mut text = String::new();
                use std::io::Read as _;
                let _ = stderr.read_to_string(&mut text);
                let text = text.trim();
                if !text.is_empty() {
                    if let Ok(mut slot) = sink.lock() {
                        *slot = Some(text.to_string());
                    }
                }
            });
    }
    let child = Arc::new(SharedChild(std::sync::Mutex::new(child)));
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, frames): (SyncSender<(i64, Arc<Frame>)>, _) =
        sync_channel(queue_depth.max(1).min(120));

    let worker_stop = stop.clone();
    let worker = std::thread::Builder::new()
        .name("otd-video-decode".into())
        .spawn(move || {
            let bytes = width as usize * height as usize * 4;
            let mut index = start_index;
            let mut buffer = vec![0u8; bytes];
            while !worker_stop.load(Ordering::Relaxed) {
                if stdout.read_exact(&mut buffer).is_err() {
                    break; // End of stream, or the process was killed.
                }
                let pixels = std::mem::replace(&mut buffer, vec![0u8; bytes]);
                let frame = Arc::new(Frame {
                    width,
                    height,
                    pixels,
                });
                // Block when the cook is behind: back-pressure is what keeps
                // a fast source from filling memory. Only a disconnect ends
                // the loop — a full queue is normal.
                let mut pending = (index, frame);
                loop {
                    match tx.try_send(pending) {
                        Ok(()) => break,
                        Err(TrySendError::Full(back)) => {
                            pending = back;
                            if worker_stop.load(Ordering::Relaxed) {
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(2));
                        }
                        Err(TrySendError::Disconnected(_)) => return,
                    }
                }
                index += 1;
            }
        })
        .map_err(|e| format!("could not start ffmpeg decode thread: {e}"))?;

    Ok(Reader {
        frames,
        stop,
        worker: Some(worker),
        child: Some(child),
        file_backend: Some(MovieDecodeBackend::Ffmpeg),
        #[cfg(target_os = "macos")]
        capture_hold: None,
        #[cfg(target_os = "macos")]
        file_decode_hold: None,
    })
}

/// Decode a still image to RGBA8.
fn decode_still(path: &Path) -> Result<Frame, String> {
    let image = image::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let rgba = image.to_rgba8();
    Ok(Frame {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}

#[cfg(test)]
mod tests {
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
        let mut source = Source {
            key: "t".into(),
            info: Info {
                width: 640,
                height: 480,
                fps: 30.0,
                duration: 10.0,
            },
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
            playback_sample_rate: 30.0,
            playback_ended: false,
            opened: None,
        };
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
        let mut source = Source {
            key: "t".into(),
            info: Info {
                width: 640,
                height: 480,
                fps: 25.0,
                duration: 10.0,
            },
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
            movie_playing: false,
            snap_index: false,
            playback_speed: 1.0,
            dropped_frames: 0,
            prefer_hw: true,
            preread: QUEUE_DEPTH,
            max_decode_cpus: 0,
            playback_index: 0,
            playback_length: 250,
            playback_sample_rate: 25.0,
            playback_ended: false,
            opened: None,
        };
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
        let mut source = Source {
            key: "t".into(),
            info: Info {
                width: 640,
                height: 480,
                fps: 24.0,
                duration: 10.0,
            },
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
            playback_length: 240,
            playback_sample_rate: 24.0,
            playback_ended: false,
            opened: None,
        };
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
    fn info_index_follows_continuous_playhead_for_chop_slope() {
        let mut source = Source {
            key: "t".into(),
            info: Info {
                width: 640,
                height: 480,
                fps: 25.0,
                duration: 10.0,
            },
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
            playback_length: 250,
            playback_sample_rate: 25.0,
            playback_ended: false,
            opened: None,
        };
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
        let mut a = Source {
            key: "a".into(),
            info: Info {
                width: 640,
                height: 480,
                fps: 30.0,
                duration: 10.0,
            },
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
            playback_sample_rate: 30.0,
            playback_ended: false,
            opened: None,
        };
        let info = Info {
            width: 640,
            height: 480,
            fps: 30.0,
            duration: 10.0,
        };
        let mut b = Source {
            key: "b".into(),
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
            playback_sample_rate: 30.0,
            playback_ended: false,
            opened: None,
        };
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
        let mut source = Source {
            key: "t".into(),
            info: Info {
                width: 640,
                height: 480,
                fps: 30.0,
                duration: 10.0,
            },
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
            playback_sample_rate: 30.0,
            playback_ended: false,
            opened: None,
        };
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
            preread: 4,
            max_decode_cpus: 0,
            playback_index: 0,
            playback_length: 500,
            playback_sample_rate: 25.0,
            playback_ended: false,
            opened: None,
        };
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
        drop(tx);
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
}
