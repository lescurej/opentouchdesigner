use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "units" => Param::menu("persecond", &["persecond", "persample"])
            .with_label("Units"),
    }
}

/// The derivative — the inverse of the Speed CHOP, which integrates.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let scale = if c.menu("units") == 0 {
        input.sample_rate.max(1.0) as f32
    } else {
        1.0
    };
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        // Carrying the last sample across the frame boundary is what keeps
        // the slope continuous; without it every frame starts with a spike.
        let mut prev = c.state.prev[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                let d = (x - prev) * scale;
                prev = *x;
                d
            })
            .collect();
        c.state.prev[ci] = prev;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "slopeCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Slope",
                &["in"],
                "The rate of change of a channel.",
                params,
                cook,
            )
}
