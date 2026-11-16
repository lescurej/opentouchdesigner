use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "operation" => Param::menu("fanout", &["fanout", "fanin"]).with_label("Operation"),
        "channels" => Param::int(4).with_label("Channels (fan out)").with_range(1.0, 64.0),
        "quantize" => Param::menu("on", &["off", "on"]).with_label("Quantize Output"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let n = input.channels.first().map_or(1, |ch| ch.samples.len()).max(1);
    let channels = if c.menu("operation") == 0 {
        let count = c.i("channels").clamp(1, 64) as usize;
        let quantize = c.menu("quantize") != 0;
        let index = input.channels.first();
        let max_idx = (count.saturating_sub(1)) as f32;
        (0..count)
            .map(|k| {
                let samples = (0..n)
                    .map(|s| {
                        let v = index.and_then(|ch| ch.samples.get(s)).copied().unwrap_or(-1.0);
                        if quantize {
                            let idx = v.floor().clamp(0.0, max_idx);
                            if idx as usize == k {
                                1.0
                            } else {
                                0.0
                            }
                        } else {
                            let lo = v.floor().clamp(0.0, max_idx);
                            let hi = (lo + 1.0).min(max_idx);
                            if k as f32 == lo {
                                1.0 - v.fract().clamp(0.0, 1.0)
                            } else if k as f32 == hi && hi > lo {
                                v.fract().clamp(0.0, 1.0)
                            } else {
                                0.0
                            }
                        }
                    })
                    .collect();
                Channel::new(format!("chan{k}"), samples)
            })
            .collect()
    } else {
        let samples = (0..n)
            .map(|s| {
                input
                    .channels
                    .iter()
                    .position(|ch| ch.samples.get(s).is_some_and(|v| *v > 0.0))
                    .map_or(-1.0, |i| i as f32)
            })
            .collect();
        vec![Channel::new("chan1", samples)]
    };
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

pub const TYPE_NAME: &str = "fanCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Fan",
        &["in"],
        "Fan out an index channel to one on/off channel per value (chan0..chanN-1), \
         or fan many channels in to the index of the first one above zero (-1 if none).",
        params,
        cook,
    )
}