use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "method" => Param::menu(
            "transpose",
            &["transpose", "reverse", "swapfirstlast", "sequence"],
        ).with_label("Method"),
    }
}

/// Rearrange the channel/sample grid.
///
/// Transpose is the one that earns its keep: N channels of one sample become
/// one channel of N samples, which is how a pile of separate values becomes
/// something a CHOP to TOP or an instancing Geometry COMP can consume.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let n = input.num_samples();
    let channels = match c.menu("method") {
        0 => {
            let rows: Vec<Channel> = (0..n)
                .map(|s| {
                    Channel::new(
                        format!("chan{}", s + 1),
                        input
                            .channels
                            .iter()
                            .map(|ch| ch.samples.get(s).copied().unwrap_or(0.0))
                            .collect(),
                    )
                })
                .collect();
            rows
        }
        1 => input
            .channels
            .iter()
            .map(|ch| {
                let mut s = ch.samples.clone();
                s.reverse();
                Channel::new(ch.name.clone(), s)
            })
            .collect(),
        2 => {
            let mut out = input.channels.clone();
            if out.len() >= 2 {
                let last = out.len() - 1;
                out.swap(0, last);
            }
            out
        }
        _ => {
            // Every channel end to end in one.
            let mut all = Vec::new();
            for ch in &input.channels {
                all.extend_from_slice(&ch.samples);
            }
            vec![Channel::new("chan1", all)]
        }
    };
    // Transposing a time-sliced stream produces a buffer, not a stream.
    ChopData::new(channels, input.sample_rate, false)
}



pub const TYPE_NAME: &str = "shuffleCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Shuffle",
                &["in"],
                "Rearrange the channel/sample grid — transpose, reverse, sequence.",
                params,
                cook,
            )
}
