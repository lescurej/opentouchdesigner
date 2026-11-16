use std::path::Path;

use crate::Geometry;

mod crypto;
mod dump;
mod geom;
mod header;
mod parse;
mod types;

#[cfg(test)]
mod tests;

pub use dump::{dump_path, dump_text};
pub use header::MAGIC;

pub(crate) use geom::geometry_from;
pub(crate) use parse::read_detail;

pub fn is_btog(data: &[u8]) -> bool {
    header::header_version(data) == Some(5)
}

pub fn load(path: &Path) -> Result<Geometry, String> {
    let data = std::fs::read(path).map_err(|e| format!("btog: {e}"))?;
    parse(&data)
}

pub fn parse(data: &[u8]) -> Result<Geometry, String> {
    geometry_from(read_detail(data)?)
}