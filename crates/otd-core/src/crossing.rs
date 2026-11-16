//! What one family's output looks like to another family's operator.
//!
//! PLAN.md §2.1 keeps the wire families separate on purpose — "explicit
//! converter ops + parameter references bridge families" — but until now only
//! the parameter half existed. This is the other half.
//!
//! The design constraint is the crate graph: `otd-chop` must not depend on
//! `otd-dat`, and `otd-gpu` must not depend on either, or the clean core
//! PLAN.md §3 asks for stops being clean. So a family's output crosses the
//! boundary in *this* neutral, dependency-free form, and the converter
//! operator — which lives in the crate of the family it produces — decides
//! what the bytes mean. `otd-engine` is the only place that fills these in,
//! for the same reason it is the only place a TOP parameter can read a CHOP.
//!
//! Deliberately raw: no naming, no scaling, no interpretation. A SOP arrives
//! as an interleaved buffer and a schema, not as channels called `P(0)`,
//! because choosing that name is the SOP to CHOP operator's job and putting it
//! here would scatter one operator's behaviour across two crates.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// A CHOP crossing, unpacked: names, samples, sample rate.
pub type ChannelView<'a> = (&'a [String], &'a [Vec<f32>], f64);

/// A CHOP→DAT table that is not materialized as strings (large CHOPs stay in the channel store).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChopTableMirror {
    pub chop_path: String,
    pub layout_columns: bool,
    pub include_names: bool,
    pub format: String,
    pub channels: usize,
    pub samples: usize,
}

/// Interleaved soundtrack filled by the movie reader while it runs.
///
/// Sample 0 is media time 0. The reader appends; playback may start as soon
/// as the first buffer is in, and a later sample is audible once the reader
/// has reached it.
pub struct MoviePcm {
    pub sample_rate: f64,
    pub channels: usize,
    samples: Mutex<Vec<f32>>,
    frames: AtomicU64,
}

impl std::fmt::Debug for MoviePcm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoviePcm")
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .field("frames", &self.frames())
            .finish()
    }
}

impl MoviePcm {
    pub fn new(sample_rate: f64, channels: usize, samples: Vec<f32>) -> Self {
        let channels = channels.max(1);
        let frames = (samples.len() / channels) as u64;
        Self {
            sample_rate,
            channels,
            samples: Mutex::new(samples),
            frames: AtomicU64::new(frames),
        }
    }

    pub fn frames(&self) -> usize {
        self.frames.load(Ordering::Acquire) as usize
    }

    pub fn push(&self, extra: &[f32]) {
        if extra.is_empty() {
            return;
        }
        let mut samples = self.samples.lock().unwrap_or_else(|e| e.into_inner());
        samples.extend_from_slice(extra);
        let frames = (samples.len() / self.channels.max(1)) as u64;
        self.frames.store(frames, Ordering::Release);
    }

    /// Linear interpolation of one frame. `out` is one sample per channel.
    pub fn read_frame(&self, pos: f64, out: &mut [f32]) {
        let frames = self.frames();
        if frames == 0 || out.is_empty() {
            out.fill(0.0);
            return;
        }
        let samples = self.samples.lock().unwrap_or_else(|e| e.into_inner());
        let ch = self.channels.max(1);
        let frames = (samples.len() / ch).min(frames);
        if frames == 0 {
            out.fill(0.0);
            return;
        }
        let last = (frames - 1) as f64;
        let pos = pos.clamp(0.0, last);
        let i0 = pos.floor() as usize;
        let frac = (pos - i0 as f64) as f32;
        let i1 = (i0 + 1).min(frames - 1);
        for (c, slot) in out.iter_mut().enumerate().take(ch) {
            let a = samples[i0 * ch + c];
            let b = samples[i1 * ch + c];
            *slot = a + (b - a) * frac;
        }
    }
}

/// Soundtrack decoded by the movie reader (AVFoundation on macOS).
///
/// `Ready` shares the sample buffer. Equality is by pointer so a cook does
/// not compare the whole file.
#[derive(Clone, Debug)]
pub enum MovieAudio {
    /// The movie is open and its audio track is still being read.
    Pending,
    /// The file has no audio track.
    Missing,
    Failed(String),
    Ready(Arc<MoviePcm>),
}

impl PartialEq for MovieAudio {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MovieAudio::Pending, MovieAudio::Pending)
            | (MovieAudio::Missing, MovieAudio::Missing) => true,
            (MovieAudio::Failed(a), MovieAudio::Failed(b)) => a == b,
            (MovieAudio::Ready(a), MovieAudio::Ready(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// One input of a foreign family, as the receiving operator sees it.
#[derive(Clone, Debug, PartialEq)]
pub enum Crossing {
    /// A DAT: rows of cells.
    Table(Vec<Vec<String>>),
    /// A CHOP: named channels, all the same length.
    Channels {
        names: Vec<String>,
        samples: Vec<Vec<f32>>,
        sample_rate: f64,
    },
    Points {
        attrs: Vec<(String, usize)>,
        data: Vec<f32>,
    },
    Pixels {
        width: u32,
        height: u32,
        rgba: Vec<f32>,
    },
    Info {
        channels: Vec<(String, f32)>,
    },
    /// Movie File In soundtrack, filled by the video engine.
    MovieAudio(MovieAudio),
}

impl Crossing {
    pub fn as_table(&self) -> Option<&[Vec<String>]> {
        match self {
            Crossing::Table(rows) => Some(rows),
            _ => None,
        }
    }

    pub fn as_channels(&self) -> Option<ChannelView<'_>> {
        match self {
            Crossing::Channels {
                names,
                samples,
                sample_rate,
            } => Some((names, samples, *sample_rate)),
            _ => None,
        }
    }

    /// Bytes per point in [`Crossing::Points`].
    pub fn point_stride(&self) -> usize {
        match self {
            Crossing::Points { attrs, .. } => attrs.iter().map(|(_, w)| *w).sum(),
            _ => 0,
        }
    }

    /// Where an attribute starts within a point, and how wide it is.
    pub fn as_info(&self) -> Option<&[(String, f32)]> {
        match self {
            Crossing::Info { channels } => Some(channels),
            _ => None,
        }
    }

    pub fn as_movie_audio(&self) -> Option<&MovieAudio> {
        match self {
            Crossing::MovieAudio(audio) => Some(audio),
            _ => None,
        }
    }

    pub fn attr(&self, name: &str) -> Option<(usize, usize)> {
        let Crossing::Points { attrs, .. } = self else {
            return None;
        };
        let mut offset = 0;
        for (n, w) in attrs {
            if n == name {
                return Some((offset, *w));
            }
            offset += w;
        }
        None
    }
}

/// The foreign inputs of one node, indexed the way its inputs are.
///
/// Sparse on purpose: an operator that mixes families — a converter that also
/// takes an input of its own kind — gets `None` for the slots its own engine
/// already handled.
pub type Crossings = Vec<Option<Crossing>>;
