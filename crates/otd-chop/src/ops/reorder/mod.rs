pub const TYPE_NAME: &str = "reorderCHOP";

use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::ChopData;

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "method" => Param::menu("numsuffix", &["numsuffix"]).with_label("Method"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let mut channels = input.channels.clone();
    if c.menu("method") == 0 {
        channels.sort_by_key(|a| suffix_key(&a.name));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

fn suffix_key(name: &str) -> (String, i64) {
    let start = name
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .last()
        .map(|(i, _)| i)
        .unwrap_or(name.len());
    let n = name[start..].parse::<i64>().unwrap_or(i64::MAX);
    (name[..start].to_string(), n)
}

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Reorder",
        &["in"],
        "Sort channels by the number at the end of the name.",
        params,
        cook,
    )
}
