use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{Channel, ChopData};
use crate::ops::ChopCtx;

use super::ChopSpec;

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "samples" => Param::int(0).with_label("Shift (samples)").with_range(-8192.0, 8192.0),
        "extend" => Param::menu("hold", &["hold", "zero"]).with_label("Extend"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let shift = c.i("samples").clamp(-8192, 8192) as isize;
    let zero = c.menu("extend") == 1;
    let channels = input
        .channels
        .iter()
        .map(|ch| {
            let len = ch.samples.len();
            let samples: Vec<f32> = (0..len)
                .map(|i| {
                    let src = i as isize - shift;
                    if src < 0 || src >= len as isize {
                        if zero {
                            0.0
                        } else {
                            ch.samples.first().copied().unwrap_or(0.0)
                        }
                    } else {
                        ch.samples[src as usize]
                    }
                })
                .collect();
            Channel::new(ch.name.clone(), samples)
        })
        .collect();
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

pub const TYPE_NAME: &str = "shiftCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Shift",
        &["in"],
        "Slide samples left or right, holding or filling with zero.",
        params,
        cook,
    )
}
