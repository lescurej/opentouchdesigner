use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "function" => Param::menu(
            "average",
            &["average", "maximum", "minimum", "rms", "sum", "length", "median"],
        ).with_label("Function"),
    }
}

/// Reduce each channel to one number.
///
/// A whole waveform is rarely what a parameter wants; one number that
/// describes it usually is. Keeping the channel names means a downstream
/// export still reads `ch('/analyze1', 'chan1')` and gets the summary.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let func = c.menu("function");
    let channels = input
        .channels
        .iter()
        .map(|ch| {
            let s = ch.as_slice();
            let n = s.len().max(1) as f32;
            let v = match func {
                0 => s.iter().sum::<f32>() / n,
                1 => ch.max(),
                2 => ch.min(),
                3 => (s.iter().map(|v| v * v).sum::<f32>() / n).sqrt(),
                4 => s.iter().sum::<f32>(),
                5 => s.len() as f32,
                _ => {
                    let mut sorted = s.to_vec();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    sorted.get(sorted.len() / 2).copied().unwrap_or(0.0)
                }
            };
            Channel::new(ch.name.clone(), vec![v])
        })
        .collect();
    ChopData::new(channels, input.sample_rate, false)
}



pub const TYPE_NAME: &str = "analyzeCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Analyze",
                &["in"],
                "Reduce each channel to one number.",
                params,
                cook,
            )
}
