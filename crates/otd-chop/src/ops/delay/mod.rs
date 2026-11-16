use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "delay" => Param::float(0.25).with_label("Delay (s)").with_range(0.0, 10.0),
        "mix" => Param::float(1.0).with_label("Mix").with_range(0.0, 1.0),
        "feedback" => Param::float(0.0).with_label("Feedback").with_range(0.0, 0.95),
    }
}

/// Hold each channel's history and read it back later.
///
/// The buffer is per channel and lives in the node's state, which is what
/// makes this a CHOP rather than a parameter trick: the delay is in *samples*
/// of the input's own rate, so the same node delays a 60 Hz control channel
/// and a 48 kHz audio channel by the same wall-clock time.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let want = ((c.f("delay") as f64) * rate).round().max(0.0) as usize;
    let mix = c.f("mix").clamp(0.0, 1.0);
    let fb = c.f("feedback").clamp(0.0, 0.95);

    if want == 0 {
        return input;
    }
    // One ring per channel, laid end to end in the state's float scratch so
    // the existing per-node state carries it with no new machinery. The ring
    // is exactly `want` long: read the oldest slot, overwrite it, then
    // advance, which makes sample *n* come out as sample *n - want* rather
    // than one late.
    let stride = want;
    let needed = input.num_channels() * stride;
    if c.state.f.len() != needed {
        c.state.f.clear();
        c.state.f.resize(needed, 0.0);
        c.state.i.clear();
        c.state.i.resize(input.num_channels().max(1), 0);
    }
    c.state.i.resize(input.num_channels().max(1), 0);

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let base = ci * stride;
        let mut head = c.state.i[ci] as usize;
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                let delayed = c.state.f[base + head];
                c.state.f[base + head] = x + delayed * fb;
                head = (head + 1) % stride;
                x * (1.0 - mix) + delayed * mix
            })
            .collect();
        c.state.i[ci] = head as i64;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "delayCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Delay",
                &["in"],
                "Play a channel back later, with optional feedback.",
                params,
                cook,
            )
}
