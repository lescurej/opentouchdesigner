use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{Channel, ChopData, slice_len};

use super::{spec as chop_spec, ChopCtx, ChopSpec};
use super::helpers::with_rate;

pub const TYPE_NAME: &str = "constantCHOP";
pub const CONSTANT: &str = TYPE_NAME;
pub const CONSTANT_MAX_ROWS: i64 = 64;

pub(crate) fn params() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "rows".into(),
        Param::int(1)
            .with_label("Rows")
            .with_page("Constant")
            .with_range(1.0, CONSTANT_MAX_ROWS as f64),
    );
    for i in 0..CONSTANT_MAX_ROWS as usize {
        let default_name = if i == 0 { "chan1" } else { "chan" };
        m.insert(
            format!("name{i}"),
            Param::str(default_name)
                .with_label("Channel Name")
                .with_page("Constant"),
        );
        m.insert(
            format!("value{i}"),
            Param::float(0.0).with_label("Value").with_page("Constant"),
        );
    }
    with_rate(m)
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let channels = if c.node.param("rows").is_some() {
        cook_constant_rows(c, n)
    } else {
        cook_constant_legacy(c, n)
    };
    ChopData::new(channels, rate, true)
}

pub(crate) fn cook_constant_legacy(c: &ChopCtx<'_>, n: usize) -> Vec<Channel> {
    let name = c.s("name");
    let count = otd_core::channel_count_from_name(&name);
    let names = otd_core::expand_channel_names(&name, count);
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| Channel::constant(name, c.f(&format!("value{i}")), n))
        .collect()
}

pub(crate) fn cook_constant_rows(c: &ChopCtx<'_>, n: usize) -> Vec<Channel> {
    let rows = c.i("rows").clamp(1, CONSTANT_MAX_ROWS) as usize;
    let mut channels = Vec::new();
    for i in 0..rows {
        let name_tpl = c.s(&format!("name{i}"));
        let val = c.f(&format!("value{i}"));
        let count = otd_core::channel_count_from_name(&name_tpl);
        let names = otd_core::expand_channel_names(&name_tpl, count);
        for name in names {
            channels.push(Channel::constant(name, val, n));
        }
    }
    channels
}

pub fn spec() -> ChopSpec {
    chop_spec(
        TYPE_NAME,
        "Constant",
        &[],
        "Fixed values as channels.",
        params,
        cook,
    )
}