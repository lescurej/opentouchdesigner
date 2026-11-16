use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "source" => Param::str("").with_label("NDI Source Name"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let _ = c.s("source");
    c.error = Some("NDI DAT requires the optional NDI SDK (not bundled).".into());
    DatData::table(vec![vec!["status".into(), "unavailable".into()]])
}
pub const TYPE_NAME: &str = "ndiDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "NDI",
        &[],
        "NDI metadata (stub).",
        params,
        cook,
    )
}
