
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::Geometry;

use super::helpers::load_geometry_path;
use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "file" => Param::str("").with_label("File (.obj / .data)").as_file_ref(),
        "geometry" => Param::str("").with_label("Geometry"),
        "mergeinput" => Param::bool(false).with_label("Merge Input"),
    }
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let _selection = c.s("geometry");
    let mut path = c.resolve_file("file");
    if path.as_os_str().is_empty() {
        path = c.resolve_data_sidecar();
    }
    let mut geo = load_geometry_path(&path);
    if c.b("mergeinput") {
        geo = geo.merged(&c.input(0));
    }
    geo
}
pub const TYPE_NAME: &str = "importselectSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Import Select",
        &["in"],
        "Load one object/group from a .obj, optionally merged with the input.",
        params,
        cook,
    )
}
