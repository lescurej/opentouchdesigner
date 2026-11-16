use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::helpers::*;
use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "convert" => Param::menu(
            "nonzero",
            &["off", "nonzero", "greater", "less", "equal"],
        ).with_label("Convert"),
        "threshold" => Param::float(0.5).with_label("Threshold"),
        "invert" => Param::bool(false).with_label("Invert"),
        "combine" => Param::menu("off", &["off", "and", "or", "xor"]).with_label("Combine Channels"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let (convert, threshold, invert, combine_op) = (
        c.menu("convert"),
        c.f("threshold"),
        c.b("invert"),
        c.menu("combine"),
    );
    let mut out = map_samples(&input, |_, _, v| {
        let on = match convert {
            0 => return v,
            2 => v > threshold,
            3 => v < threshold,
            4 => (v - threshold).abs() < 1e-6,
            _ => v != 0.0,
        };
        let on = on != invert;
        if on { 1.0 } else { 0.0 }
    });

    if combine_op > 0 && out.num_channels() > 1 {
        let len = out.num_samples();
        let samples = (0..len)
            .map(|i| {
                let mut bits = out.channels.iter().map(|c| c.samples[i] != 0.0);
                let result = match combine_op {
                    1 => bits.all(|b| b),
                    2 => bits.any(|b| b),
                    _ => bits.fold(false, |a, b| a ^ b),
                };
                if result { 1.0 } else { 0.0 }
            })
            .collect();
        out = ChopData::new(
            vec![Channel::new("logic", samples)],
            out.sample_rate,
            out.time_sliced,
        );
    }
    out
}



pub const TYPE_NAME: &str = "logicCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Logic",
                &["in"],
                "Turn channels into on/off and combine them.",
                params,
                cook,
            )
}
