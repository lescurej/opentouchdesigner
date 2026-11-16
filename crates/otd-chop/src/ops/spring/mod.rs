use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "spring" => Param::float(80.0).with_label("Spring Constant").with_range(0.0, 1000.0),
        "damping" => Param::float(6.0).with_label("Damping").with_range(0.0, 100.0),
    }
}

/// Follow the input like a mass on a spring: overshoot, wobble, settle.
///
/// Unlike Lag this keeps moving after the input stops, so it has to cook
/// every frame. Position is `state.f`, velocity is `state.prev`, and
/// `state.i` marks a channel as started so the first cook begins at rest on
/// the input instead of flying in from zero.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let dt = 1.0 / input.sample_rate.max(1.0) as f32;
    let (k, d) = (c.f("spring").max(0.0), c.f("damping").max(0.0));
    c.state.fit(input.num_channels());
    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        if c.state.i[ci] == 0 {
            c.state.f[ci] = ch.samples.first().copied().unwrap_or(0.0);
            c.state.prev[ci] = 0.0;
            c.state.i[ci] = 1;
        }
        let (mut y, mut v) = (c.state.f[ci], c.state.prev[ci]);
        let samples = ch
            .samples
            .iter()
            .map(|x| {
                // Semi-implicit Euler: stable for the stiffness range offered.
                v += (k * (x - y) - d * v) * dt;
                y += v * dt;
                y
            })
            .collect();
        (c.state.f[ci], c.state.prev[ci]) = (y, v);
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "springCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Spring",
                &["in"],
                "Follow a channel like a mass on a spring: overshoot, wobble, then settle. \
                 Raise damping to stop the wobble; raise the spring constant to follow faster.",
                params,
                cook,
            )
}
