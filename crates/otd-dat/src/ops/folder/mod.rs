use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "folder" => Param::str("").with_label("Root Folder").as_folder_ref(),
        "names" => Param::str("*").with_label("Names"),
        "type" => Param::menu("filesandfolders", &["files", "folders", "filesandfolders"]).with_label("Type"),
        "recursive" => Param::bool(false).with_label("Include Subfolders"),
        "mindepth" => Param::int(0).with_label("Minimum Depth").with_range(0.0, 32.0),
        "limitdepth" => Param::bool(false).with_label("Limit Depth"),
        "maxdepth" => Param::int(1).with_label("Maximum Depth").with_range(0.0, 32.0),
        "dateformat" => Param::menu("std", &["std", "epoch"]).with_label("Date Format"),
        "namecol" => Param::bool(true).with_label("Name"),
        "basenamecol" => Param::bool(false).with_label("Base Name"),
        "extensioncol" => Param::bool(false).with_label("Extension"),
        "typecol" => Param::bool(true).with_label("Type"),
        "sizecol" => Param::bool(false).with_label("Size"),
        "depthcol" => Param::bool(false).with_label("Depth"),
        "foldercol" => Param::bool(false).with_label("Folder"),
        "pathcol" => Param::bool(false).with_label("Path"),
        "relpathcol" => Param::bool(false).with_label("Relative Path"),
        "datecreatedcol" => Param::bool(false).with_label("Date Created"),
        "datemodifiedcol" => Param::bool(false).with_label("Date Modified"),
        "dateaccessedcol" => Param::bool(false).with_label("Date Accessed"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let dir = c.graph.resolve_external(&c.s("folder"));
    let names = {
        let primary = c.s("names");
        if primary.trim().is_empty() {
            c.node
                .param("pattern")
                .map(|p| p.eval(c.eval).as_str())
                .unwrap_or(primary)
        } else {
            primary
        }
    };
    let cols = crate::folder::column_flags(
        c.b("namecol"),
        c.b("basenamecol"),
        c.b("extensioncol"),
        c.b("typecol"),
        c.b("sizecol"),
        c.b("depthcol"),
        c.b("foldercol"),
        c.b("pathcol"),
        c.b("relpathcol"),
        c.b("datecreatedcol"),
        c.b("datemodifiedcol"),
        c.b("dateaccessedcol"),
    );
    let cfg = crate::folder::scan_config(
        &dir,
        &names,
        c.b("recursive"),
        c.val("mindepth").as_i64().clamp(0, 32) as usize,
        c.b("limitdepth"),
        c.val("maxdepth").as_i64().clamp(0, 32) as usize,
        c.menu("type"),
        c.menu("dateformat") == 1,
    );
    match crate::folder::cook(&dir, &cfg, cols, c.b("active")) {
        Ok(table) => table,
        Err(e) => {
            c.error = Some(format!("folder: {e}"));
            crate::folder::cook(&dir, &cfg, cols, false).unwrap_or_default()
        }
    }
}
pub const TYPE_NAME: &str = "folderDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Folder",
        &[],
        "List folder.",
        params,
        cook,
    )
}
