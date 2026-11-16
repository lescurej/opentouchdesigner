use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "threshold" => Param::float(0.5).with_label("Trigger Threshold").with_range(-10.0, 10.0),
    }
}

/// Sample and hold: input 0's value is frozen until input 1 rises past the
/// threshold. Two inputs rather than a parameter because the thing that says
/// "now" is nearly always another channel — a beat, a MIDI note, a button.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let trigger = c.input(1).clone();
    let threshold = c.f("threshold");
    c.state.fit(input.num_channels() + 1);
    let n = input.num_samples();

    // Sample index of every rising edge in this slice.
    let mut fires = Vec::new();
    if let Some(t) = trigger.channels.first() {
        let mut prev = c.state.prev[input.num_channels()];
        for (i, v) in t.samples.iter().enumerate() {
            if prev < threshold && *v >= threshold {
                fires.push(i);
            }
            prev = *v;
        }
        c.state.prev[input.num_channels()] = prev;
    }

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut held = c.state.f[ci];
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            if fires.contains(&i) {
                held = ch.samples.get(i).copied().unwrap_or(held);
            }
            out.push(held);
        }
        c.state.f[ci] = held;
        channels.push(Channel::new(ch.name.clone(), out));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "holdCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Hold",
                &["in", "trigger"],
                "Freeze input 1's value until input 2 fires.",
                params,
                cook,
            )
}
