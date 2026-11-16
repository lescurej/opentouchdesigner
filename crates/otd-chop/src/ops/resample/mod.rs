use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "length" => Param::int(64).with_label("Length (samples)").with_range(1.0, 8192.0),
        "interp" => Param::menu("linear", &["linear", "nearest"]).with_label("Interpolate"),
    }
}

/// Change how many samples a buffer has, keeping its shape.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let n = c.i("length").clamp(1, 8192) as usize;
    let nearest = c.menu("interp") == 1;
    let channels = input
        .channels
        .iter()
        .map(|ch| {
            let src = &ch.samples;
            let samples = (0..n)
                .map(|i| {
                    if src.is_empty() {
                        return 0.0;
                    }
                    if src.len() == 1 {
                        return src[0];
                    }
                    let pos = i as f32 * (src.len() - 1) as f32 / (n.max(2) - 1) as f32;
                    let lo = pos.floor() as usize;
                    if nearest {
                        return src[pos.round() as usize % src.len()];
                    }
                    let hi = (lo + 1).min(src.len() - 1);
                    let f = pos - lo as f32;
                    src[lo] * (1.0 - f) + src[hi] * f
                })
                .collect();
            Channel::new(ch.name.clone(), samples)
        })
        .collect();
    // A resampled buffer is no longer one frame's worth of a stream.
    ChopData::new(channels, input.sample_rate, false)
}



pub const TYPE_NAME: &str = "resampleCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Resample",
                &["in"],
                "Change how many samples a buffer has, keeping its shape.",
                params,
                cook,
            )
}
