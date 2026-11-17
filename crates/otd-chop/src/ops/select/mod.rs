use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};
use crate::ops::helpers::wired_or_referenced_input;
use std::sync::Arc;

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
    let input = wired_or_referenced_input(c, 0, "chop");
    let pattern = c.s("channames");
    let rename = c.s("renameto");
    let pattern_empty = pattern.trim().is_empty();
    let rename_empty = rename.is_empty();

    if pattern_empty && rename_empty {
        if c.inputs.get(0).filter(|d| !d.channels.is_empty()).is_some() {
            c.passthrough_input = Some(0);
            return ChopData::empty();
        }
        return ChopData::shared_from(input);
    }

    let channels: Vec<Channel> = input
        .channels
        .iter()
        .filter(|ch| pattern_empty || otd_core::pattern_matches_channel(&pattern, &ch.name))
        .enumerate()
        .map(|(i, ch)| {
            Channel::shared(
                otd_core::rename_to_at(&rename, i, &ch.name),
                Arc::clone(&ch.samples),
            )
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