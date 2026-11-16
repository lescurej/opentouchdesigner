use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "op" => Param::str("..").with_label("Operator").as_path_ref(),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let Some(info) = c.foreign.first().and_then(|x| x.as_ref()) else {
        return ChopData::empty();
    };
    let Some(pairs) = info.as_info() else {
        return ChopData::empty();
    };
    c.state.fit(pairs.len());
    let channels: Vec<Channel> = pairs
        .iter()
        .enumerate()
        .map(|(ci, (name, value))| {
            let end = *value;
            c.state.prev[ci] = end;
            Channel::new(name.clone(), vec![end])
        })
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}


pub const TYPE_NAME: &str = "infoCHOP";
pub const INFO: &str = TYPE_NAME;

pub fn spec() -> ChopSpec {
    super::spec_animated(
        TYPE_NAME,
        "Info",
        &[],
        "Another operator's info channels (TouchDesigner Info CHOP).",
        params,
        cook,
    )
}
