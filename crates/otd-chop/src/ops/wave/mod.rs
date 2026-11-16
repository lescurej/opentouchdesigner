use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::ops::lfo::wave;

use crate::data::{Channel, ChopData, MAX_SLICE};
use crate::ops::ChopCtx;
use crate::ops::helpers::with_rate;

use super::ChopSpec;

pub(crate) fn params() -> IndexMap<String, Param> {
    let max_len = MAX_SLICE as f64;
    with_rate(params! {
        "type" => Param::menu("sine", &["sine", "triangle", "ramp", "square", "pulse", "gaussian"])
            .with_label("Type"),
        "length" => Param::int(64).with_label("Length (samples)").with_range(1.0, max_len),
        "periods" => Param::float(1.0).with_label("Periods").with_range(0.1, 64.0),
        "amplitude" => Param::float(1.0).with_label("Amplitude").with_range(0.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-2.0, 2.0),
        "phase" => Param::float(0.0).with_label("Phase").with_range(0.0, 1.0),
        "pulsewidth" => Param::float(0.5).with_label("Pulse Width").with_range(0.0, 1.0),
        "name" => Param::str("chan1").with_label("Channel Name"),
    })
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let len = c.i("length").clamp(1, MAX_SLICE as i64) as usize;
    let (kind, periods, amp, offset, phase, pw) = (
        c.menu("type"),
        c.f("periods"),
        c.f("amplitude"),
        c.f("offset"),
        c.f("phase"),
        c.f("pulsewidth"),
    );
    let samples: Vec<f32> = (0..len)
        .map(|i| {
            let x = i as f32 / len as f32;
            let p = x * periods + phase;
            let v = match kind {
                2 => p - p.floor(),
                5 => {
                    let g = (x - 0.5) * 6.0;
                    (-0.5 * g * g).exp()
                }
                1 => wave(1, p, pw),
                3 => wave(3, p, pw),
                4 => wave(4, p, pw),
                _ => wave(0, p, pw),
            };
            v * amp + offset
        })
        .collect();
    let name_tpl = c.s("name");
    let count = otd_core::channel_count_from_name(&name_tpl);
    let names = otd_core::expand_channel_names(&name_tpl, count);
    let channels: Vec<Channel> = names
        .into_iter()
        .map(|name| Channel::new(name, samples.clone()))
        .collect();
    ChopData::new(channels, rate, false)
}

pub const TYPE_NAME: &str = "waveCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Wave",
        &[],
        "One cycle of a sine, triangle, ramp, square, pulse or gaussian.",
        params,
        cook,
    )
}
