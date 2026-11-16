use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local};

use crate::DatData;

#[derive(Clone, Debug)]
struct Entry {
    name: String,
    basename: String,
    extension: String,
    type_str: String,
    size: String,
    depth: usize,
    folder: String,
    path: String,
    relpath: String,
    created: Option<SystemTime>,
    modified: Option<SystemTime>,
    accessed: Option<SystemTime>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ColumnFlags {
    name: bool,
    basename: bool,
    extension: bool,
    type_col: bool,
    size: bool,
    depth: bool,
    folder: bool,
    path: bool,
    relpath: bool,
    date_created: bool,
    date_modified: bool,
    date_accessed: bool,
}

pub(crate) struct ScanConfig<'a> {
    root: &'a Path,
    names_pattern: &'a str,
    recursive: bool,
    min_depth: usize,
    max_depth: Option<usize>,
    include_files: bool,
    include_folders: bool,
    epoch_dates: bool,
}

pub(crate) fn cook(
    root: &Path,
    cfg: &ScanConfig<'_>,
    cols: ColumnFlags,
    active: bool,
) -> Result<DatData, String> {
    let header = build_header(cols);
    if !active {
        return Ok(DatData::table(vec![header]));
    }
    if root.as_os_str().is_empty() {
        return Ok(DatData::table(vec![header]));
    }
    if !root.is_dir() {
        return Err(format!("not a directory `{}`", root.display()));
    }
    let mut entries = Vec::new();
    scan_dir(cfg.root, cfg.root, 0, &mut entries, cfg);
    let mut rows = vec![header];
    for e in entries {
        rows.push(entry_row(&e, cols, cfg.epoch_dates));
    }
    Ok(DatData::table(rows))
}

fn build_header(cols: ColumnFlags) -> Vec<String> {
    let mut h = Vec::new();
    if cols.name {
        h.push("name".into());
    }
    if cols.basename {
        h.push("basename".into());
    }
    if cols.extension {
        h.push("extension".into());
    }
    if cols.type_col {
        h.push("type".into());
    }
    if cols.size {
        h.push("size".into());
    }
    if cols.depth {
        h.push("depth".into());
    }
    if cols.folder {
        h.push("folder".into());
    }
    if cols.path {
        h.push("path".into());
    }
    if cols.relpath {
        h.push("relpath".into());
    }
    if cols.date_created {
        h.push("datecreated".into());
    }
    if cols.date_modified {
        h.push("datemodified".into());
    }
    if cols.date_accessed {
        h.push("dateaccessed".into());
    }
    if h.is_empty() {
        h.push("name".into());
    }
    h
}

fn entry_row(e: &Entry, cols: ColumnFlags, epoch_dates: bool) -> Vec<String> {
    let mut row = Vec::new();
    if cols.name {
        row.push(e.name.clone());
    }
    if cols.basename {
        row.push(e.basename.clone());
    }
    if cols.extension {
        row.push(e.extension.clone());
    }
    if cols.type_col {
        row.push(e.type_str.clone());
    }
    if cols.size {
        row.push(e.size.clone());
    }
    if cols.depth {
        row.push(e.depth.to_string());
    }
    if cols.folder {
        row.push(e.folder.clone());
    }
    if cols.path {
        row.push(e.path.clone());
    }
    if cols.relpath {
        row.push(e.relpath.clone());
    }
    if cols.date_created {
        row.push(format_time(e.created, epoch_dates));
    }
    if cols.date_modified {
        row.push(format_time(e.modified, epoch_dates));
    }
    if cols.date_accessed {
        row.push(format_time(e.accessed, epoch_dates));
    }
    row
}

fn format_time(t: Option<SystemTime>, epoch: bool) -> String {
    let Some(t) = t else {
        return String::new();
    };
    if epoch {
        return t
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64().to_string())
            .unwrap_or_default();
    }
    DateTime::<Local>::from(t)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn scan_dir(root: &Path, current: &Path, depth: usize, out: &mut Vec<Entry>, cfg: &ScanConfig<'_>) {
    if let Some(max) = cfg.max_depth {
        if depth > max {
            return;
        }
    }
    let Ok(read_dir) = fs::read_dir(current) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        let is_dir = path.is_dir();
        if is_dir && !cfg.include_folders && !cfg.recursive {
            continue;
        }
        if !is_dir && !cfg.include_files {
            if cfg.recursive && is_dir {
                scan_dir(root, &path, depth + 1, out, cfg);
            }
            continue;
        }
        if depth >= cfg.min_depth
            && ((is_dir && cfg.include_folders) || (!is_dir && cfg.include_files))
        {
            if let Some(e) = entry_from_path(root, &path, depth, is_dir) {
                if name_matches(cfg.names_pattern, &e.name) {
                    out.push(e);
                }
            }
        }
        if cfg.recursive && is_dir {
            scan_dir(root, &path, depth + 1, out, cfg);
        }
    }
}

fn entry_from_path(root: &Path, path: &Path, depth: usize, is_dir: bool) -> Option<Entry> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let relpath = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let folder = path
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let full = path.to_string_lossy().replace('\\', "/");
    let (basename, extension) = if is_dir {
        (name.clone(), String::new())
    } else {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.clone());
        let ext = path
            .extension()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        (stem, ext)
    };
    let md = fs::metadata(path).ok();
    let size = if is_dir {
        String::new()
    } else {
        md.as_ref().map(|m| m.len().to_string()).unwrap_or_default()
    };
    let type_str = if is_dir {
        String::new()
    } else if extension.is_empty() {
        "File".into()
    } else {
        extension.to_ascii_uppercase()
    };
    let (created, modified, accessed) = md
        .map(|m| (m.created().ok(), m.modified().ok(), m.accessed().ok()))
        .unwrap_or((None, None, None));
    Some(Entry {
        name,
        basename,
        extension,
        type_str,
        size,
        depth,
        folder,
        path: full,
        relpath,
        created,
        modified,
        accessed,
    })
}

fn name_matches(pattern: &str, name: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern == "*" {
        return true;
    }
    for token in pattern.split(|c: char| c.is_whitespace() || c == ',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if glob_token(token, name) {
            return true;
        }
    }
    false
}

fn glob_token(pattern: &str, name: &str) -> bool {
    if pattern.starts_with('"') && pattern.ends_with('"') && pattern.len() >= 2 {
        return &pattern[1..pattern.len() - 1] == name;
    }
    match pattern.split_once('*') {
        None => pattern == name,
        Some(("", suffix)) => name.ends_with(suffix),
        Some((prefix, "")) => name.starts_with(prefix),
        Some((prefix, suffix)) => name.starts_with(prefix) && name.ends_with(suffix),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn column_flags(
    namecol: bool,
    basenamecol: bool,
    extensioncol: bool,
    typecol: bool,
    sizecol: bool,
    depthcol: bool,
    foldercol: bool,
    pathcol: bool,
    relpathcol: bool,
    datecreatedcol: bool,
    datemodifiedcol: bool,
    dateaccessedcol: bool,
) -> ColumnFlags {
    ColumnFlags {
        name: namecol,
        basename: basenamecol,
        extension: extensioncol,
        type_col: typecol,
        size: sizecol,
        depth: depthcol,
        folder: foldercol,
        path: pathcol,
        relpath: relpathcol,
        date_created: datecreatedcol,
        date_modified: datemodifiedcol,
        date_accessed: dateaccessedcol,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn scan_config<'a>(
    root: &'a Path,
    names_pattern: &'a str,
    recursive: bool,
    min_depth: usize,
    limit_depth: bool,
    max_depth: usize,
    contents_menu: usize,
    epoch_dates: bool,
) -> ScanConfig<'a> {
    let (include_files, include_folders) = match contents_menu {
        1 => (false, true),
        2 => (true, true),
        _ => (true, false),
    };
    ScanConfig {
        root,
        names_pattern,
        recursive,
        min_depth,
        max_depth: if limit_depth { Some(max_depth) } else { None },
        include_files,
        include_folders,
        epoch_dates,
    }
}

#[cfg(test)]
#[path = "tests/folder__tests.rs"]
mod tests;
