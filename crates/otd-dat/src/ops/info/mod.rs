use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::callbacks::resolve_dat_op;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "op" => Param::str("").with_label("Operator").as_path_ref(),
        "language" => Param::menu("text", &["text", "python", "glsl"]).with_label("Language"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let mut rows = vec![vec!["name".into(), "value".into()]];
    let Some(id) = resolve_dat_op(c, &c.s("op")) else {
        rows.push(vec!["error".into(), "operator not found".into()]);
        return DatData::table(rows);
    };
    let node = c.graph.node(id);
    let wired = node.inputs.iter().filter(|slot| slot.is_some()).count();
    rows.push(vec!["path".into(), c.graph.path(id)]);
    rows.push(vec!["type".into(), node.op_type.clone()]);
    rows.push(vec!["family".into(), node.family.suffix().into()]);
    rows.push(vec!["name".into(), node.name.clone()]);
    rows.push(vec!["inputs".into(), wired.to_string()]);
    DatData::table(rows)
}

pub const TYPE_NAME: &str = "infoDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Info",
        &[],
        "Operator metadata as a table.",
        params,
        cook,
    )
}
