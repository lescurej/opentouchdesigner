use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::Geometry;

use super::helpers::{load_geometry_path, try_locked_sidecar_geometry};
use super::{SopCtx, SopSpec, spec as sop_spec};

pub const TYPE_NAME: &str = "fileinSOP";

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "file" => Param::str("").with_label("File (.obj / .data)").as_file_ref(),
    }
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    if let Some(geo) = try_locked_sidecar_geometry(c) {
        return geo;
    }
    let path = c.resolve_file("file");
    let geo = load_geometry_path(&path);
    if !geo.is_empty() {
        return geo;
    }
    load_geometry_path(&c.resolve_data_sidecar())
}

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "File In",
        &[],
        "Load triangle mesh from a Wavefront .obj file.",
        params,
        cook,
    )
}