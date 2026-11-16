use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData, slice_len, slice_times};

use super::helpers::*;
use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    with_rate(params! {
        "tempo" => Param::float(120.0).with_label("Tempo (BPM)").with_range(20.0, 300.0),
        "beatsperbar" => Param::int(4).with_label("Beats Per Bar").with_range(1.0, 32.0),
    })
}

/// A tempo clock: `ramp` 0..1 within a beat, `pulse` at each beat, `count`,
/// and the same for a bar.
///
/// Derived from the timeline's absolute time rather than from a counter, so a
/// dropped frame does not put the beat permanently behind — the same reason
/// the Animation CHOP and Movie File In are functions of time.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let bps = (c.f("tempo") as f64 / 60.0).max(1e-6);
    let per_bar = c.i("beatsperbar").max(1) as f64;

    let mut ramp = Vec::with_capacity(n);
    let mut pulse = Vec::with_capacity(n);
    let mut count = Vec::with_capacity(n);
    let mut bar_ramp = Vec::with_capacity(n);
    let mut prev = c.state.f.first().copied().unwrap_or(0.0);
    c.state.fit(1);

    for t in slice_times(c.time, n) {
        let beats = t * bps;
        let frac = beats.rem_euclid(1.0) as f32;
        ramp.push(frac);
        // A pulse is one sample wide at the wrap, which is what a Trigger or
        // a Count downstream is looking for.
        pulse.push(if frac < prev { 1.0 } else { 0.0 });
        prev = frac;
        count.push(beats.floor() as f32);
        bar_ramp.push((beats / per_bar).rem_euclid(1.0) as f32);
    }
    c.state.f[0] = prev;

    ChopData::new(
        vec![
            Channel::new("ramp", ramp),
            Channel::new("pulse", pulse),
            Channel::new("count", count),
            Channel::new("bar", bar_ramp),
        ],
        rate,
        true,
    )
}



pub const TYPE_NAME: &str = "beatCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Beat",
                &[],
                "A tempo clock: ramp, pulse, count and bar.",
                params,
                cook,
            )
}
