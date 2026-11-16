use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "threshold" => Param::float(0.5).with_label("Threshold"),
        "edge" => Param::menu("rising", &["rising", "falling", "both"]).with_label("Edge"),
        "reset" => Param::bool(false).with_label("Reset"),
        "limit" => Param::menu("off", &["off", "clamp", "wrap"]).with_label("Limit"),
        "min" => Param::float(0.0).with_label("Min"),
        "max" => Param::float(8.0).with_label("Max"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let (threshold, edge, reset, limit, lo, hi) = (
        c.f("threshold"),
        c.menu("edge"),
        c.b("reset"),
        c.menu("limit"),
        c.f("min"),
        c.f("max"),
    );
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut count = if reset { 0.0 } else { c.state.f[ci] };
        let mut prev = c.state.prev[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                let rising = prev <= threshold && *x > threshold;
                let falling = prev > threshold && *x <= threshold;
                let hit = match edge {
                    1 => falling,
                    2 => rising || falling,
                    _ => rising,
                };
                prev = *x;
                if hit {
                    count += 1.0;
                    match limit {
                        1 => count = count.clamp(lo.min(hi), hi.max(lo)),
                        2 => {
                            let span = (hi - lo).abs().max(1.0);
                            count = lo + (count - lo).rem_euclid(span);
                        }
                        _ => {}
                    }
                }
                count
            })
            .collect();
        c.state.f[ci] = count;
        c.state.prev[ci] = prev;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "countCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Count",
                &["in"],
                "Count threshold crossings.",
                params,
                cook,
            )
}
