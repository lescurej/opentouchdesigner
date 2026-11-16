use std::sync::Arc;

use super::super::Frame;
use super::reader::Source;
use super::{MovieDecodeBackend, SEEK_AHEAD_LIMIT};

impl Source {
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

    pub(crate) fn best_pending_upto(&mut self, wanted: i64) -> Option<(i64, Arc<Frame>)> {
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

    pub(crate) fn pending_limit(&self) -> usize {
        self.preread.clamp(1, 120)
    }

    pub(crate) fn push_pending(&mut self, index: i64, frame: Arc<Frame>) -> bool {
        if index <= self.current.max(0) || self.pending_frames.len() >= self.pending_limit() {
            return false;
        }
        self.pending_frames.push_back((index, frame));
        true
    }

    pub(crate) fn restart_reader(&mut self, path: &str, at: i64) {
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

    pub(crate) fn needs_seek(&self, wanted: i64, prev_wanted: i64) -> bool {
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

    pub fn cue_sequential_to(&mut self, index: i64, abs_time: f64) {
        self.playhead = index as f64;
        self.last_sequential_abs_time = abs_time;
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


}
