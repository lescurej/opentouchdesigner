use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "type" => Param::menu("lowpass", &["lowpass", "movingaverage"]).with_label("Type"),
        "width" => Param::float(0.2).with_label("Width (s)").with_range(0.0, 4.0),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let kind = c.menu("type");
    let width = c.f("width").max(0.0);
    c.state.fit(input.num_channels());

    if kind == 1 {
        // Moving average over the window, using the running value as the
        // history that predates this slice.
        let window = ((width as f64 * rate).round() as usize).max(1);
        let mut channels = Vec::with_capacity(input.num_channels());
        for (ci, ch) in input.channels.iter().enumerate() {
            let mut history = vec![c.state.f[ci]; window];
            let mut cursor = 0usize;
            let samples: Vec<f32> = ch
                .samples
                .iter()
                .map(|x| {
                    history[cursor] = *x;
                    cursor = (cursor + 1) % window;
                    history.iter().sum::<f32>() / window as f32
                })
                .collect();
            c.state.f[ci] = ch.last();
            channels.push(Channel::new(ch.name.clone(), samples));
        }
        return ChopData::new(channels, input.sample_rate, input.time_sliced);
    }

    let alpha = crate::ops::lag::lag_alpha(width, rate);
    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut y = c.state.f[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                y += (x - y) * alpha;
                y
            })
            .collect();
        c.state.f[ci] = y;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "filterCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Filter",
                &["in"],
                "Low-pass or moving-average smoothing.",
                params,
                cook,
            )
}
