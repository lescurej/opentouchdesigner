use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData, slice_len, slice_times};

use super::helpers::*;
use super::{ChopCtx, ChopSpec};


/// The starting keys in a new Animation CHOP: a one-second ramp, so the
/// operator animates something the moment it is created.
const DEFAULT_KEYS: &str = "\
# channel  time  value  interpolation
chan1      0     0      smooth
chan1      1     1      smooth
";

pub(crate) fn params() -> IndexMap<String, Param> {
    with_rate(params! {
        "keys" => Param::str(DEFAULT_KEYS).with_label("Keys"),
        "play" => Param::menu("timeline", &["timeline", "loop", "hold"]).with_label("Play"),
        "speed" => Param::float(1.0).with_label("Speed").with_range(-4.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-60.0, 60.0),
    })
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let (curves, _) = crate::anim::Curves::parse(&c.s("keys"));

    let mode = c.menu("play");
    let (speed, offset) = (c.f("speed") as f64, c.f("offset") as f64);
    // `loop` needs the keyed span; without keys there is nothing to wrap into
    // and the curve is flat anyway.
    let span = curves.range().filter(|(lo, hi)| hi > lo);

    let at = |t: f64| -> f64 {
        let t = t * speed + offset;
        match (mode, span) {
            // 1 == loop. Wrapping with rem_euclid rather than `%` so it works
            // going backwards too, which a negative speed does.
            (1, Some((lo, hi))) => lo + (t - lo).rem_euclid(hi - lo),
            _ => t,
        }
    };

    // One sample per slice sample, so an exported parameter reads the value at
    // *this* instant and a downstream filter sees a continuous signal — the
    // same time-slicing contract every other generator here keeps.
    let times: Vec<f64> = slice_times(c.time, n).map(at).collect();
    let channels = curves
        .0
        .iter()
        .map(|(name, curve)| {
            Channel::new(
                name.clone(),
                times.iter().map(|t| curve.sample(*t)).collect(),
            )
        })
        .collect::<Vec<_>>();

    // A CHOP with no channels at all is awkward downstream; an animation with
    // no keys yet reports one silent channel rather than nothing.
    let channels = if channels.is_empty() {
        vec![Channel::new("chan1".to_string(), vec![0.0; n])]
    } else {
        channels
    };
    ChopData::new(channels, rate, true)
}



pub const TYPE_NAME: &str = "animationCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Animation",
                &[],
                "Keyframed curves over time.",
                params,
                cook,
            )
}
