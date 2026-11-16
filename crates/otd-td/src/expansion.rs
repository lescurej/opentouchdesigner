use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExpansionError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// Read a `toeexpand` directory into path → bytes (paths use `/`).
pub fn load_dir(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, ExpansionError> {
    let mut files = BTreeMap::new();
    load_dir_inner(root, root, &mut files)?;
    Ok(files)
}

fn load_dir_inner(
    root: &Path,
    here: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), ExpansionError> {
    for entry in fs::read_dir(here)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            load_dir_inner(root, &path, files)?;
        } else if meta.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let key = rel.to_string_lossy().replace('\\', "/");
            let data = fs::read(&path)?;
            files.insert(key, data);
        }
    }
    Ok(())
}

pub fn read_touchdesigner_file(path: &Path) -> Result<Vec<u8>, ExpansionError> {
    Ok(fs::read(path)?)
}
