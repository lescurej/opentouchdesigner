use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "type" => Param::menu("ramp", &["ramp", "sine", "triangle", "square", "gaussian"])
            .with_label("Type"),
        "length" => Param::int(64).with_label("Length").with_range(1.0, 4096.0),
        "periods" => Param::float(1.0).with_label("Periods").with_range(0.1, 32.0),
        "amplitude" => Param::float(1.0).with_label("Amplitude").with_range(0.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-2.0, 2.0),
        "phase" => Param::float(0.0).with_label("Phase").with_range(0.0, 1.0),
        "name" => Param::str("chan1").with_label("Channel Name"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let len = c.i("length").clamp(1, 4096) as usize;
    let (kind, periods, amp, offset, phase) = (
        c.menu("type"),
        c.f("periods"),
        c.f("amplitude"),
        c.f("offset"),
        c.f("phase"),
    );
    let samples: Vec<f32> = (0..len)
        .map(|i| {
            let x = i as f32 / len as f32;
            let p = x * periods + phase;
            let v = match kind {
                0 => p - p.floor(),
                4 => {
                    let g = (x - 0.5) * 6.0;
                    (-0.5 * g * g).exp()
                }
                other => super::lfo::wave(other, p, 0.5),
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
    ChopData::new(channels, CONTROL_RATE, false)
}



pub const TYPE_NAME: &str = "patternCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Pattern",
                &[],
                "A fixed-length waveform buffer.",
                params,
                cook,
            )
}
