use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "gain" => Param::float(1.0).with_label("Gain").with_range(-4.0, 4.0),
        "initial" => Param::float(0.0).with_label("Initial Value"),
        "reset" => Param::bool(false).with_label("Reset"),
        "limit" => Param::menu("off", &["off", "clamp", "wrap"]).with_label("Limit"),
        "min" => Param::float(0.0).with_label("Min"),
        "max" => Param::float(1.0).with_label("Max"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let dt = 1.0 / rate as f32;
    let (gain, initial, reset, limit, lo, hi) = (
        c.f("gain"),
        c.f("initial"),
        c.b("reset"),
        c.menu("limit"),
        c.f("min"),
        c.f("max"),
    );
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut pos = if reset { initial } else { c.state.f[ci] };
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|v| {
                pos += v * gain * dt;
                match limit {
                    1 => pos = pos.clamp(lo.min(hi), hi.max(lo)),
                    2 => {
                        let span = (hi - lo).abs().max(1e-6);
                        pos = lo + (pos - lo).rem_euclid(span);
                    }
                    _ => {}
                }
                pos
            })
            .collect();
        c.state.f[ci] = pos;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "speedCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Speed",
                &["in"],
                "Integrate a channel: velocity becomes position.",
                params,
                cook,
            )
}
