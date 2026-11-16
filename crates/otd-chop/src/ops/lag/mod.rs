use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "lagup" => Param::float(0.1).with_label("Lag Up (s)").with_range(0.0, 2.0),
        "lagdown" => Param::float(0.1).with_label("Lag Down (s)").with_range(0.0, 2.0),
    }
}

/// One-pole coefficient for a time constant, per sample.
pub(crate) fn lag_alpha(tau: f32, sample_rate: f64) -> f32 {
    if tau <= 1e-5 {
        return 1.0;
    }
    let dt = 1.0 / sample_rate.max(1.0) as f32;
    1.0 - (-dt / tau).exp()
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let (up, down) = (c.f("lagup"), c.f("lagdown"));
    let (a_up, a_down) = (lag_alpha(up, rate), lag_alpha(down, rate));
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut y = c.state.f[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                let alpha = if *x > y { a_up } else { a_down };
                y += (x - y) * alpha;
                y
            })
            .collect();
        c.state.f[ci] = y;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "lagCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Lag",
                &["in"],
                "Smooth a channel, with separate rise and fall times.",
                params,
                cook,
            )
}
