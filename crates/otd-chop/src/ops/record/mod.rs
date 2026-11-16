use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{Channel, ChopData, CONTROL_RATE, MAX_SLICE};
use crate::io::RecordBuffer;
use crate::ops::ChopCtx;

use super::{spec_animated, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "record" => Param::bool(false).with_label("Record"),
        "clear" => Param::bool(false).with_label("Clear"),
        "maxlength" => Param::float(10.0).with_label("Length (seconds)").with_range(0.01, 600.0),
    }
}

fn positive_rate(rate: f64) -> f64 {
    if rate.is_finite() && rate > 0.0 {
        rate
    } else {
        CONTROL_RATE
    }
}

/// How many samples `seconds` is at `sample_rate`. `None` when the length is
/// not a real duration, so a NaN parameter cannot drain the buffer to nothing.
fn record_length_samples(seconds: f32, sample_rate: f64) -> Option<usize> {
    const MAX_SAMPLES: f64 = 30_000_000.0;
    if !seconds.is_finite() || !sample_rate.is_finite() || sample_rate <= 0.0 {
        return None;
    }
    let seconds = f64::from(seconds).clamp(0.01, 600.0);
    let n = (seconds * sample_rate).round();
    Some(n.clamp(1.0, MAX_SAMPLES) as usize)
}

fn record_len(buf: &RecordBuffer) -> usize {
    buf.channels.first().map(|c| c.samples.len()).unwrap_or(0)
}

/// Stretch `samples` so they cover the same duration at `to_rate`.
fn resample_channel(samples: &[f32], from_rate: f64, to_rate: f64) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let same = !from_rate.is_finite()
        || !to_rate.is_finite()
        || from_rate <= 0.0
        || to_rate <= 0.0
        || (from_rate - to_rate).abs() < 1.0;
    if same {
        return samples.to_vec();
    }
    let n = ((samples.len() as f64 / from_rate) * to_rate).round();
    let n = n.clamp(1.0, MAX_SLICE as f64) as usize;
    if n == samples.len() {
        return samples.to_vec();
    }
    let last = samples.len() - 1;
    (0..n)
        .map(|i| {
            if last == 0 || n == 1 {
                return samples[0];
            }
            let pos = i as f64 * last as f64 / (n - 1) as f64;
            let i0 = (pos.floor() as usize).min(last);
            let i1 = (i0 + 1).min(last);
            let t = (pos - i0 as f64) as f32;
            samples[i0] + (samples[i1] - samples[i0]) * t
        })
        .collect()
}

/// Add this cook's samples. Channels are matched by name: a new name is
/// padded with silence, a name missing this frame holds its last sample.
fn append_record(buf: &mut RecordBuffer, input: &ChopData) {
    let incoming: Vec<(String, Vec<f32>)> = input
        .channels
        .iter()
        .map(|ch| {
            (
                ch.name.clone(),
                resample_channel(&ch.samples, input.sample_rate, buf.sample_rate),
            )
        })
        .collect();
    let add = incoming.first().map(|(_, s)| s.len()).unwrap_or(0);
    if add == 0 {
        return;
    }
    let have = record_len(buf);
    for (name, _) in &incoming {
        if !buf.channels.iter().any(|c| &c.name == name) {
            buf.channels
                .push(Channel::new(name.clone(), vec![0.0; have]));
        }
    }
    for ch in &mut buf.channels {
        if let Some((_, samples)) = incoming.iter().find(|(name, _)| name == &ch.name) {
            ch.samples.extend_from_slice(samples);
        } else {
            let hold = ch.samples.last().copied().unwrap_or(0.0);
            ch.samples.resize(ch.samples.len() + add, hold);
        }
    }
}

fn trim_record(buf: &mut RecordBuffer, max: usize) {
    for ch in &mut buf.channels {
        if ch.samples.len() > max {
            let drop = ch.samples.len() - max;
            ch.samples.drain(..drop);
        }
    }
}

/// Append one time slice per cook while Record is on, and keep that history
/// until Clear turns on.
///
/// The buffer is keyed by node id and keeps the sample rate of the first
/// slice stored in it. A later slice at another rate is resampled into that
/// rate; a cook with no channels leaves the buffer alone. A non-sliced input
/// is a whole clip restated every frame, so it is taken once, while the
/// buffer is empty. Clear wipes on the rising edge only.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let id = c.id;
    let record = c.b("record");
    let clear = c.b("clear");
    let seconds = c.f("maxlength");
    if c.state.prev.is_empty() {
        c.state.prev.push(0.0);
    }
    let clear_pulse = clear && c.state.prev[0] < 0.5;
    c.state.prev[0] = if clear { 1.0 } else { 0.0 };
    if clear_pulse {
        c.io.record_buffers.remove(&id);
    }

    let usable = !input.channels.is_empty() && input.num_samples() > 0;
    let stored = c.io.record_buffers.get(&id).map(record_len).unwrap_or(0);
    if record && usable && (input.time_sliced || stored == 0) {
        let rate = positive_rate(input.sample_rate);
        let buf =
            c.io.record_buffers
                .entry(id)
                .or_insert_with(|| RecordBuffer {
                    sample_rate: rate,
                    channels: Vec::new(),
                });
        if record_len(buf) == 0 {
            buf.sample_rate = rate;
        }
        append_record(buf, &input);
    }

    if let Some(buf) = c.io.record_buffers.get_mut(&id) {
        if let Some(max) = record_length_samples(seconds, buf.sample_rate) {
            trim_record(buf, max);
        }
    }

    match c.io.record_buffers.get(&id) {
        Some(buf) if !buf.channels.is_empty() => {
            ChopData::new(buf.channels.clone(), positive_rate(buf.sample_rate), false)
        }
        _ => input,
    }
}

pub const TYPE_NAME: &str = "recordCHOP";

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Record",
        &["in"],
        "Appends each time slice while Record is on, until Clear or the length limit.",
        params,
        cook,
    )
}
