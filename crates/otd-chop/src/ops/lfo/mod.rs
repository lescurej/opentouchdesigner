use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData, slice_len, slice_times};

use super::helpers::*;
use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    with_rate(params! {
        "type" => Param::menu("sine", &["sine", "triangle", "ramp", "square", "pulse"])
            .with_label("Type"),
        "frequency" => Param::float(1.0).with_label("Frequency").with_range(0.0, 20.0),
        "amplitude" => Param::float(1.0).with_label("Amplitude").with_range(0.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-2.0, 2.0),
        "phase" => Param::float(0.0).with_label("Phase").with_range(0.0, 1.0),
        "pulsewidth" => Param::float(0.5).with_label("Pulse Width").with_range(0.0, 1.0),
        "name" => Param::str("lfo").with_label("Channel Name"),
    })
}

pub(crate) fn wave(kind: usize, phase: f32, pulse_width: f32) -> f32 {
    let p = phase - phase.floor();
    match kind {
        1 => 1.0 - 4.0 * (p - 0.5).abs(),
        2 => p,
        3 => {
            if p < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        4 => {
            if p < pulse_width.clamp(0.0, 1.0) {
                1.0
            } else {
                0.0
            }
        }
        _ => (p * std::f32::consts::TAU).sin(),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let (kind, freq, amp, offset, phase, pw) = (
        c.menu("type"),
        c.f("frequency"),
        c.f("amplitude"),
        c.f("offset"),
        c.f("phase"),
        c.f("pulsewidth"),
    );
    let samples: Vec<f32> = slice_times(c.time, n)
        .map(|t| wave(kind, t as f32 * freq + phase, pw) * amp + offset)
        .collect();
    let name_tpl = c.s("name");
    let count = otd_core::channel_count_from_name(&name_tpl);
    let names = otd_core::expand_channel_names(&name_tpl, count);
    let channels: Vec<Channel> = names
        .into_iter()
        .map(|name| Channel::new(name, samples.clone()))
        .collect();
    ChopData::new(channels, rate, true)
}



pub const TYPE_NAME: &str = "lfoCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "LFO",
                &[],
                "A repeating waveform over time.",
                params,
                cook,
            )
}
