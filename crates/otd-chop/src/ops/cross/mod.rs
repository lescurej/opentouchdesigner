use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "cross" => Param::float(0.5).with_label("Cross").with_range(0.0, 1.0),
        "curve" => Param::menu("linear", &["linear", "equalpower", "smooth"])
            .with_label("Curve"),
    }
}

/// Blend two inputs channel by channel.
///
/// Equal power is the default worth knowing about: a linear crossfade between
/// two uncorrelated signals dips ~3 dB in the middle, which is audible on
/// audio and visible on anything driving brightness.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let a = c.input(0).clone();
    let b = c.input(1).clone();
    let t = c.f("cross").clamp(0.0, 1.0);
    let quarter = std::f32::consts::FRAC_PI_2;
    let (wa, wb) = match c.menu("curve") {
        1 => (((1.0 - t) * quarter).sin(), (t * quarter).sin()),
        2 => {
            let s = t * t * (3.0 - 2.0 * t);
            (1.0 - s, s)
        }
        _ => (1.0 - t, t),
    };

    let len = a.num_samples().max(b.num_samples());
    let count = a.num_channels().max(b.num_channels());
    let channels = (0..count)
        .map(|ci| {
            let ca = a.nth(ci);
            let cb = b.nth(ci);
            let name = ca
                .or(cb)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| format!("chan{}", ci + 1));
            let samples = (0..len)
                .map(|i| {
                    let va = ca.map(|c| sample_held(&c.samples, i)).unwrap_or(0.0);
                    let vb = cb.map(|c| sample_held(&c.samples, i)).unwrap_or(0.0);
                    va * wa + vb * wb
                })
                .collect();
            Channel::new(name, samples)
        })
        .collect();
    ChopData::new(channels, a.sample_rate.max(b.sample_rate), a.time_sliced)
}

/// Read a sample, holding the last one past the end.
///
/// This is what lets a single-sample control channel broadcast against a
/// waveform — the same rule instancing uses, so it does not surprise.
fn sample_held(s: &[f32], i: usize) -> f32 {
    s.get(i).or_else(|| s.last()).copied().unwrap_or(0.0)
}



pub const TYPE_NAME: &str = "crossCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Cross",
                &["a", "b"],
                "Blend two inputs, with an equal-power option.",
                params,
                cook,
            )
}
