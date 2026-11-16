use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "from" => Param::str("*").with_label("Rename Channels"),
        "to" => Param::str("chan[1-]").with_label("To"),
    }
}

/// Rename channels, TD's numbered-pattern style: `chan[1-]` names them
/// `chan1`, `chan2`, … A plain name with no bracket renames every match to
/// the same thing, which is what you want feeding a Merge.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let from = c.s("from");
    let to = c.s("to");
    let patterns: Vec<&str> = from.split_whitespace().collect();

    let numbered = otd_core::parse_numbered_pattern(&to);
    let mut rename_index = 0usize;
    let channels = input
        .channels
        .iter()
        .map(|ch| {
            if !patterns.iter().any(|p| otd_core::pattern_matches_channel(p, &ch.name)) {
                return ch.clone();
            }
            let name = if let Some(pat) = numbered {
                let n = otd_core::channel_name_at(&pat, rename_index);
                rename_index += 1;
                n
            } else {
                to.clone()
            };
            Channel::new(name, ch.samples.clone())
        })
        .collect();
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "renameCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Rename",
                &["in"],
                "Rename channels by pattern.",
                params,
                cook,
            )
}
