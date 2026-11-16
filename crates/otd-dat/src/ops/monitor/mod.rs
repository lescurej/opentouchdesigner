use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {}
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    c.net.monitor.table()
}
pub const TYPE_NAME: &str = "monitorDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "Monitor",
        &[],
        "Network monitor.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
