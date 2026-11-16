//! Partial TouchDesigner expansion → OpenTouchDesigner graph import.
use std::path::{Path, PathBuf};

use otd_core::NodeId;

use crate::DecodeSource;
use crate::DecodedExpansion;

#[derive(Debug, thiserror::Error)]
pub enum ImportGraphError {
    #[error("could not determine expansion root folder")]
    NoRootPrefix,
    #[error("missing root descriptor `{0}.n`")]
    NoRootDescriptor(String),
    #[error("unsupported operator in root: {0}")]
    BadRootType(String),
    #[error("{0}")]
    Registry(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct ImportGraphResult {
    pub root: NodeId,
    pub nodes: usize,
    pub wires: usize,
    pub warnings: Vec<String>,
}

pub fn expansion_filesystem_root(
    path: &Path,
    decode: &DecodedExpansion,
) -> Result<PathBuf, ImportGraphError> {
    match &decode.source {
        DecodeSource::ToeExpand { dir, .. } => Ok(dir.clone()),
        DecodeSource::Native(_) => {
            let dir = crate::expansion_dir(path);
            std::fs::create_dir_all(&dir)?;
            for (rel, data) in &decode.files {
                let dest = dir.join(rel);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(dest, data)?;
            }
            Ok(dir)
        }
    }
}