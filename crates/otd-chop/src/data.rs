//! What a CHOP produces: named channels of samples.
//!
//! PLAN.md §2.1 — CHOPs are "channels of samples", the control and audio
//! family. The load-bearing detail is §4's **time slicing**: a CHOP that is
//! generating over time emits only the samples covering the interval since the
//! last frame, sized from the *real* elapsed time. A frame that takes 33 ms
//! produces twice the samples of one that takes 16 ms, so an LFO stays on
//! pitch and audio stays continuous when the renderer stutters.

use std::sync::Arc;

use otd_core::CookContext;

pub type SampleBuf = Arc<Vec<f32>>;

/// Control CHOP sample rate (Hz). Matches TD's default; independent of
/// project timeline fps — a 24 fps show still time-slices control CHOPs here.
pub const CONTROL_RATE: f64 = 60.0;

/// A safety bound on one frame's slice, so a stalled frame cannot allocate
/// unboundedly.
pub const MAX_SLICE: usize = 1 << 16;

#[derive(Clone, Debug)]
pub struct Channel {
    pub name: String,
    pub samples: SampleBuf,
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.samples.as_slice() == other.samples.as_slice()
    }
}

impl Channel {
    pub fn new(name: impl Into<String>, samples: Vec<f32>) -> Self {
        Channel {
            name: name.into(),
            samples: Arc::new(samples),
        }
    }

    pub fn shared(name: impl Into<String>, samples: SampleBuf) -> Self {
        Channel {
            name: name.into(),
            samples,
        }
    }

    pub fn from_vec(name: impl Into<String>, samples: Vec<f32>) -> Self {
        Channel::new(name, samples)
    }

    /// A channel holding one repeated value.
    pub fn constant(name: impl Into<String>, value: f32, len: usize) -> Self {
        Channel::from_vec(name, vec![value; len.max(1)])
    }

    pub fn as_slice(&self) -> &[f32] {
        self.samples.as_slice()
    }

    pub fn copy_samples(&self) -> Vec<f32> {
        self.as_slice().to_vec()
    }

    pub fn samples_mut(&mut self) -> &mut Vec<f32> {
        Arc::make_mut(&mut self.samples)
    }

    /// The most recent sample — what a parameter export reads.
    pub fn last(&self) -> f32 {
        self.samples.last().copied().unwrap_or(0.0)
    }

    pub fn min(&self) -> f32 {
        self.as_slice()
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
    }

    pub fn max(&self) -> f32 {
        self.as_slice()
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChopData {
    pub channels: Vec<Channel>,
    pub sample_rate: f64,
    /// True when this is one frame's worth of a continuous stream, rather
    /// than a standalone buffer like a Pattern CHOP's waveform.
    pub time_sliced: bool,
}

impl ChopData {
    pub fn new(channels: Vec<Channel>, sample_rate: f64, time_sliced: bool) -> Self {
        ChopData {
            channels,
            sample_rate,
            time_sliced,
        }
    }

    pub fn empty() -> Self {
        ChopData {
            channels: Vec::new(),
            sample_rate: CONTROL_RATE,
            time_sliced: true,
        }
    }

    /// Samples per channel. Channels are always the same length within one
    /// CHOP; operators that combine inputs pad to the longest.
    pub fn num_samples(&self) -> usize {
        self.channels.first().map(|c| c.samples.len()).unwrap_or(0)
    }

    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }

    pub fn channel(&self, name: &str) -> Option<&Channel> {
        self.channels.iter().find(|c| c.name == name)
    }

    /// The current value of a channel by name — the Export-mode read.
    pub fn value(&self, name: &str) -> Option<f32> {
        self.channel(name).map(|c| c.last())
    }

    /// Channel by index, tolerating an out-of-range index.
    pub fn nth(&self, index: usize) -> Option<&Channel> {
        self.channels.get(index)
    }

    pub fn names(&self) -> Vec<String> {
        self.channels.iter().map(|c| c.name.clone()).collect()
    }

    /// Make every channel the same length by repeating the last sample.
    pub fn pad_to(&mut self, len: usize) {
        for c in &mut self.channels {
            let last = c.last();
            if c.samples.len() < len {
                c.samples_mut().resize(len, last);
            }
        }
    }

    pub fn shared_from(input: &ChopData) -> Self {
        ChopData {
            channels: input
                .channels
                .iter()
                .map(|ch| Channel::shared(ch.name.clone(), Arc::clone(&ch.samples)))
                .collect(),
            sample_rate: input.sample_rate,
            time_sliced: input.time_sliced,
        }
    }
}

/// How many samples cover the time that has actually elapsed.
///
/// Never zero: a CHOP that produced no samples would leave downstream
/// operators — and any parameter exporting from it — with nothing to read.
pub fn slice_len(sample_rate: f64, ctx: &CookContext) -> usize {
    let n = (sample_rate * ctx.dt.max(0.0)).round() as i64;
    n.clamp(1, MAX_SLICE as i64) as usize
}

/// The time of each sample in a slice, ending at the current frame time.
///
/// Sample `n-1` lands exactly on `ctx.time`, so the value a parameter reads
/// is the value *now* rather than one slice in the past.
pub fn slice_times(ctx: &CookContext, n: usize) -> impl Iterator<Item = f64> + use<> {
    let step = if n > 0 { ctx.dt / n as f64 } else { 0.0 };
    let start = ctx.time - ctx.dt;
    (0..n).map(move |i| start + step * (i + 1) as f64)
}

/// Absolute transport time for each sample in a slice (pairs with [`slice_times`]).
pub fn slice_abs_times(ctx: &CookContext, n: usize) -> impl Iterator<Item = f64> + use<> {
    let step = if n > 0 { ctx.dt / n as f64 } else { 0.0 };
    let start = ctx.abs_time - ctx.dt;
    (0..n).map(move |i| start + step * (i + 1) as f64)
}

#[cfg(test)]
#[path = "tests/data__tests.rs"]
mod tests;
