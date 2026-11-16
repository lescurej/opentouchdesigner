use otd_core::indexmap::IndexMap;
use otd_core::Param;

use std::sync::Arc;

use crate::chopfile;
use crate::data::{Channel, ChopData, CONTROL_RATE};
use crate::ops::ChopCtx;

use super::ChopSpec;

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "file" => Param::str("").with_label("File (.och)").as_file_ref(),
    }
}

fn file_in_silent() -> Arc<ChopData> {
    Arc::new(ChopData::new(
        vec![Channel::constant("chan1", 0.0, 1)],
        CONTROL_RATE,
        false,
    ))
}

pub(crate) fn cook_file_in_shared(c: &mut ChopCtx) -> Arc<ChopData> {
    let file = c.s("file");
    let path = c.path.to_string();
    if file.trim().is_empty() {
        c.io.chop_files.remove(&path);
        c.io.clear_note(&path);
        return file_in_silent();
    }
    let stale =
        c.io.chop_files
            .get(&path)
            .map(|(f, _)| f != &file)
            .unwrap_or(true);
    if stale {
        let resolved = chopfile::resolve_path(c.io.base_dir.as_deref(), &file);
        match crate::bclip::load_any(&resolved) {
            Ok(data) => {
                c.io.clear_note(&path);
                c.io.chop_files.insert(path.clone(), (file, Arc::new(data)));
            }
            Err(e) => {
                c.io.note(&path, e);
                c.io.chop_files.remove(&path);
            }
        }
    }
    c.io.chop_files
        .get(&path)
        .map(|(_, d)| Arc::clone(d))
        .unwrap_or_else(file_in_silent)
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let arc = cook_file_in_shared(c);
    Arc::try_unwrap(arc).unwrap_or_else(|a| a.as_ref().clone())
}

pub const TYPE_NAME: &str = "fileinCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "File In",
        &[],
        "Load a .och or .bclip channel file.",
        params,
        cook,
    )
}
