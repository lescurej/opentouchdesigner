use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "source" => Param::str(
            "# Leave a list of lists in `rows`.\nrows = [['frame'], [frame]]\n",
        ).with_label("Source").as_script(),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let source = c.s("source");
    let Some(host) = c.scripts else {
        c.error = Some("no Python interpreter in this build".into());
        return DatData::default();
    };
    match host.run_table(&source, c.eval, c.path) {
        Ok(rows) => DatData::table(rows),
        Err(e) => {
            c.error = Some(e);
            DatData::default()
        }
    }
}

// ------------------------------------------------------------------- UDP
pub const TYPE_NAME: &str = "scriptDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Script",
        &[],
        "Python table generator.",
        params,
        cook,
    )
}
