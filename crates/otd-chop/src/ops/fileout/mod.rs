use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::chopfile;
use crate::data::ChopData;
use crate::ops::ChopCtx;

use super::ChopSpec;

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "file" => Param::str("capture.och").with_label("File (.och)").as_file_ref(),
        "write" => Param::bool(true).with_label("Write"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    if c.b("write") && !input.channels.is_empty() {
        let file = c.s("file");
        if !file.trim().is_empty() {
            let resolved = chopfile::resolve_path(c.io.base_dir.as_deref(), &file);
            if let Err(e) = chopfile::save(&resolved, &input) {
                c.io.note(c.path, e);
            } else {
                c.io.clear_note(c.path);
            }
        }
    }
    input
}

pub const TYPE_NAME: &str = "fileoutCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "File Out",
        &["in"],
        "Write the input channels to a .och file.",
        params,
        cook,
    )
}
