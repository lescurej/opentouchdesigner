use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};
use crate::ops::helpers::wired_or_referenced_chop;

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "chop" => Param::str("").with_label("CHOP").as_path_ref(),
        "channames" => Param::str("*").with_label("Channel Names"),
        "renamefrom" => Param::str("").with_label("Rename From"),
        "renameto" => Param::str("").with_label("Rename To"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = wired_or_referenced_chop(c, 0, "chop");
    let pattern = c.s("channames");
    let rename = c.s("renameto");

    let channels: Vec<Channel> = input
        .channels
        .iter()
        .filter(|ch| pattern.trim().is_empty() || otd_core::pattern_matches_channel(&pattern, &ch.name))
        .enumerate()
        .map(|(i, ch)| Channel {
            name: otd_core::rename_to_at(&rename, i, &ch.name),
            samples: ch.samples.clone(),
        })
        .collect();
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

pub const TYPE_NAME: &str = "selectCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Select",
        &["in"],
        "Pick and rename channels.",
        params,
        cook,
    )
}