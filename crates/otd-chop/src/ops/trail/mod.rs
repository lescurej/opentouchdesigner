use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "window" => Param::float(2.0).with_label("Window (s)").with_range(0.05, 60.0),
        "reset" => Param::bool(false).with_label("Reset"),
        "resetpulse" => Param::bool(false).with_label("Reset Pulse"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let len = ((c.f("window") as f64 * rate).round() as usize).clamp(2, 1 << 16);
    let needed = input.num_channels() * len;
    if c.state.f.len() != needed {
        c.state.f = vec![0.0; needed];
        c.state.i = vec![0; input.num_channels().max(1)];
    }
    if c.b("resetpulse") || c.b("reset") {
        c.state.f.fill(0.0);
        c.state.i.fill(0);
        let zeros = vec![0.0; len];
        let channels = input
            .channels
            .iter()
            .map(|ch| Channel::new(ch.name.clone(), zeros.clone()))
            .collect();
        return ChopData::new(channels, input.sample_rate, false);
    }
    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let ring = &mut c.state.f[ci * len..(ci + 1) * len];
        let mut head = c.state.i[ci] as usize;
        for x in &ch.samples {
            ring[head] = *x;
            head = (head + 1) % len;
        }
        c.state.i[ci] = head as i64;
        let samples = ring[head..].iter().chain(&ring[..head]).copied().collect();
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, false)
}

pub const TYPE_NAME: &str = "trailCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
        TYPE_NAME,
        "Trail",
        &["in"],
        "The last few seconds of each channel as one buffer, oldest sample first — \
         draw a waveform, or lay out one instance per sample.",
        params,
        cook,
    )
}