use std::fs;
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "file" => Param::str("out.txt").with_label("File").as_file_ref(),
        "append" => Param::bool(false).with_label("Append"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let path = c.graph.resolve_external(&c.s("file"));
    let text = c.input(0).as_text();
    let key = format!("{}\0{}", path.display(), c.b("append"));
    let prev = c.watched.values.get(&key).copied().unwrap_or(0.0);
    let hash = text.len() as f64 + if c.b("append") { 1.0 } else { 0.0 };
    if (prev - hash).abs() < f64::EPSILON && !text.is_empty() {
        return c.input(0);
    }
    if !text.is_empty() {
        let result = if c.b("append") {
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(text.as_bytes())
                })
        } else if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
            fs::write(&path, text.as_bytes())
        } else {
            fs::write(&path, text.as_bytes())
        };
        if let Err(e) = result {
            c.error = Some(format!("file out: {e}"));
        } else {
            c.watched.values.insert(key, hash);
        }
    }
    c.input(0)
}
pub const TYPE_NAME: &str = "fileoutDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "File Out",
        &["in"],
        "Write text file.",
        params,
        cook,
    )
}
