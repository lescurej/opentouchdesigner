use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::time::Instant;

use super::super::decode::{
    decode_still, ffmpeg_command, missing_ffmpeg, Info,
};
use super::super::device::{camera_input, camera_modes, choose_mode};
#[cfg(target_os = "macos")]
use super::super::macos;

use super::reader::Source;
use super::{spawn_ffmpeg, QUEUE_DEPTH, Reader};

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

    pub fn stream(key: String, url: &str) -> Result<Source, String> {
        let url = url.trim();
        if url.is_empty() {
            return Err("no stream URL".into());
        }
        let mut command = ffmpeg_command().ok_or_else(missing_ffmpeg)?;
        command.args(["-hide_banner", "-v", "error", "-i", url]);
        let problem: Arc<std::sync::Mutex<Option<String>>> = Default::default();
        let reader = spawn_ffmpeg(command, 1280, 720, 0, problem.clone(), QUEUE_DEPTH)?;
        Ok(Source {
            key,
            info: Info {
                width: 1280,
                height: 720,
                fps: 30.0,
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
            playback_sample_rate: 30.0,
            playback_ended: false,
            opened: Some(Instant::now()),
        })
    }

    pub fn screen(
        key: String,
        screen: u32,
        width: u32,
        height: u32,
        fps: f64,
    ) -> Result<Source, String> {
        let fps = fps.clamp(1.0, 120.0);
        let (chosen_w, chosen_h) = (width.max(1), height.max(1));
        let mut command = ffmpeg_command().ok_or_else(missing_ffmpeg)?;
        command.args(["-hide_banner", "-v", "error", "-framerate", &format!("{fps}")]);
        #[cfg(target_os = "macos")]
        command.args([
            "-f",
            "avfoundation",
            "-i",
            &format!("{screen}:none"),
            "-video_size",
            &format!("{chosen_w}x{chosen_h}"),
        ]);
        #[cfg(target_os = "windows")]
        command.args(["-f", "gdigrab", "-i", "desktop"]);
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        command.args(["-f", "x11grab", "-i", ":0.0"]);
        let problem: Arc<std::sync::Mutex<Option<String>>> = Default::default();
        let reader = spawn_ffmpeg(command, chosen_w, chosen_h, 0, problem.clone(), QUEUE_DEPTH)?;
        Ok(Source {
            key,
            info: Info {
                width: chosen_w,
                height: chosen_h,
                fps,
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
            playback_sample_rate: fps.max(1e-6),
            playback_ended: false,
            opened: Some(Instant::now()),
        })
    }


}
