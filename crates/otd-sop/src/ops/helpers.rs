use otd_core::Param;
use otd_core::indexmap::IndexMap;

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}

pub(crate) fn params_missing() -> IndexMap<String, Param> {
    otd_core::missing::missing_params()
}

pub(crate) fn no_params() -> IndexMap<String, Param> {
    params! {}
}

use crate::{Geometry};

use super::SopCtx;

pub(crate) fn load_geometry_path(path: &std::path::Path) -> Geometry {
    if path.as_os_str().is_empty() || !path.is_file() {
        return Geometry::default();
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "obj" => crate::obj::load(path, "").unwrap_or_default(),
        "data" => crate::btog::load(path).unwrap_or_default(),
        _ => {
            if let Ok(bytes) = std::fs::read(path) {
                if crate::btog::is_btog(&bytes) {
                    return crate::btog::parse(&bytes).unwrap_or_default();
                }
            }
            Geometry::default()
        }
    }
}

pub(crate) fn normalise(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-8 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

pub(crate) fn try_locked_sidecar_geometry(c: &SopCtx) -> Option<Geometry> {
    if !c.node.flags.lock {
        return None;
    }
    let geo = load_geometry_path(&c.resolve_data_sidecar());
    if geo.is_empty() { None } else { Some(geo) }
}
