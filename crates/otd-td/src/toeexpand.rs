//! Run Derivative's `toeexpand` when native decode is unavailable.

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToeExpandError {
    #[error("toeexpand not found — install TouchDesigner or add it to PATH")]
    NotFound,
    #[error("toeexpand failed: {0}")]
    Failed(String),
    #[error("expected expansion directory missing: {0}")]
    NoOutput(PathBuf),
}

/// Where TD writes the tree for `file.tox` → `file.tox.dir`.
pub fn expansion_dir(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".dir");
    PathBuf::from(s)
}

pub fn find_toeexpand() -> Option<PathBuf> {
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join("toeexpand");
            #[cfg(windows)]
            let candidate = dir.join("toeexpand.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let apps = Path::new("/Applications");
        let Ok(entries) = std::fs::read_dir(apps) else {
            return None;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("TouchDesigner") {
                continue;
            }
            let root = entry.path();
            for candidate in [
                root.join("TouchDesigner.app/Contents/MacOS/toeexpand"),
                root.join("bin/toeexpand"),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        let bundled = Path::new("/Applications/TouchDesigner.app/Contents/MacOS/toeexpand");
        if bundled.is_file() {
            return Some(bundled.to_path_buf());
        }
    }
    #[cfg(target_os = "windows")]
    {
        let base = Path::new(r"C:\Program Files\Derivative");
        if let Ok(entries) = std::fs::read_dir(base) {
            let mut dirs: Vec<_> = entries.flatten().collect();
            dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            for entry in dirs {
                let candidate = entry.path().join("bin").join("toeexpand.exe");
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Expand in place beside `path`. TouchDesigner returns exit code 1 on success.
pub fn run_toeexpand(path: &Path) -> Result<PathBuf, ToeExpandError> {
    let tool = find_toeexpand().ok_or(ToeExpandError::NotFound)?;
    let output = Command::new(&tool)
        .arg(path)
        .output()
        .map_err(|e| ToeExpandError::Failed(e.to_string()))?;
    if !output.status.success() && !output.status.code().is_some_and(|c| c == 1) {
        let msg = String::from_utf8_lossy(&output.stderr);
        return Err(ToeExpandError::Failed(msg.trim().to_string()));
    }
    let dir = expansion_dir(path);
    if !dir.is_dir() {
        return Err(ToeExpandError::NoOutput(dir));
    }
    let _ = tool;
    Ok(dir)
}

/// Path to the `toeexpand` binary that was found, if any.
pub fn toeexpand_tool() -> Option<PathBuf> {
    find_toeexpand()
}
