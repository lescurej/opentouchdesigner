use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{ChopData, CONTROL_RATE, slice_len};
use crate::ops::ChopCtx;
use crate::timecode;

use super::ltcout::ltc_clock;
use super::timecode::smpte_channels;
use super::{spec_animated, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "ratemenu" => Param::menu("30", &["24", "25", "29.97", "30", "60"]).with_label("FPS"),
        "dropframe" => Param::bool(false).with_label("Drop Frame"),
        "rate" => Param::float(CONTROL_RATE).with_label("Output Rate").with_range(1.0, 48000.0),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let (fps, drop) = ltc_clock(&c.s("ratemenu"), c.b("dropframe"));
    let path = c.path.to_string();
    let out_rate = c.rate().max(1.0);
    let n = slice_len(out_rate, c.time);

    let audio = input
        .channels
        .first()
        .filter(|ch| !ch.samples.is_empty() && input.sample_rate >= 4000.0);
    if let Some(ch) = audio {
        let cap = ((input.sample_rate / fps) * 6.0).round() as usize;
        let cap = cap.clamp(4096, 96_000);
        c.io.push_ltc_audio(&path, &ch.samples, cap, input.sample_rate);
        let peak =
            c.io.ltc_audio(&path)
                .map(|ring| ring.iter().fold(0.0f32, |m, s| m.max(s.abs())))
                .unwrap_or(0.0);
        if peak < 1.0e-4 {
            c.io.clear_ltc(&path);
            c.io.clear_note(&path);
        } else if let Some(ring) = c.io.ltc_audio(&path) {
            if let Some(smpte) = timecode::decode_ltc_stream(ring, input.sample_rate, fps, drop) {
                c.io.ltc_decode.insert(path.clone(), smpte);
                c.io.clear_note(&path);
            } else if !c.io.ltc_decode.contains_key(&path) {
                c.io.note(&path, "no LTC lock");
            }
        }
    } else if input.channels.iter().any(|ch| !ch.samples.is_empty()) {
        c.io.clear_ltc(&path);
        c.io.note(&path, "LTC needs an audio-rate input");
    } else {
        c.io.clear_ltc(&path);
        c.io.clear_note(&path);
    }

    let smpte =
        c.io.ltc_decode
            .get(&path)
            .cloned()
            .unwrap_or_else(|| timecode::smpte_from_seconds(0.0, fps, drop));
    ChopData::new(smpte_channels(&smpte, n), out_rate, true)
}

pub const TYPE_NAME: &str = "ltcinCHOP";

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "LTC In",
        &["in"],
        "Decode linear timecode audio into hour, minute, second and frame.",
        params,
        cook,
    )
}
