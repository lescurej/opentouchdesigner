//! Open DAT bodies in an external text editor or spreadsheet app.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use otd_core::{Family, NodeId, Value};
use otd_dat::ops::{SCRIPT, TABLE, TEXT};

use crate::app::OtdApp;

#[derive(Clone, Debug)]
pub(crate) struct ExternalDatWatch {
    path: PathBuf,
    mtime: SystemTime,
}

pub fn default_editor_command() -> String {
    #[cfg(target_os = "macos")]
    {
        "open".into()
    }
    #[cfg(target_os = "windows")]
    {
        "notepad".into()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "xdg-open".into()
    }
}

fn edit_dir() -> PathBuf {
    otd_ai::keys::config_dir().join("dat_external")
}

fn safe_filename(path: &str) -> String {
    path.trim_start_matches('/')
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn content_param(op_type: &str) -> &'static str {
    if op_type == SCRIPT { "source" } else { "text" }
}

fn file_extension(op_type: &str) -> &'static str {
    match op_type {
        SCRIPT => "py",
        TABLE => "tsv",
        TEXT => "txt",
        _ => "txt",
    }
}

pub fn dat_body_for_export(app: &OtdApp, id: NodeId) -> Option<String> {
    if app.graph.node(id).family != Family::Dat {
        return None;
    }
    let op = app.graph.node(id).op_type.as_str();
    let key = content_param(op);
    Some(app.graph.node(id).params[key].value.as_str())
}

fn edit_path(app: &OtdApp, id: NodeId) -> PathBuf {
    let op = app.graph.node(id).op_type.as_str();
    let ext = file_extension(op);
    let name = safe_filename(&app.graph.path(id));
    edit_dir().join(format!("{name}.{ext}"))
}

fn shell_escape_posix(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// `%path%` in settings, case-insensitive; tolerate a few common typos.
fn normalize_path_placeholder(command: &str) -> String {
    let mut s = command.to_string();
    if s.contains("%PATH%") {
        s = s.replace("%PATH%", "%path%");
    }
    if s.contains("%Path%") {
        s = s.replace("%Path%", "%path%");
    }
    // Wrong quote / diaeresis after "path" (e.g. %path¨% from a bad paste).
    for bad in ["%path¨%", "%path\"%"] {
        if s.contains(bad) {
            s = s.replace(bad, "%path%");
        }
    }
    s
}

fn spawn_editor(command: &str, path: &Path) -> Result<(), String> {
    let command = command.trim();
    let path_str = path.display().to_string();
    let cmd_line = if command.is_empty() {
        default_editor_command()
    } else {
        normalize_path_placeholder(command)
    };

    let shell_line = if cmd_line.contains("%path%") {
        cmd_line.replace("%path%", &shell_escape_posix(&path_str))
    } else if cmd_line.split_whitespace().count() <= 1 {
        format!(
            "{} {}",
            shell_escape_posix(&cmd_line),
            shell_escape_posix(&path_str)
        )
    } else {
        format!("{} {}", cmd_line.trim(), shell_escape_posix(&path_str))
    };

    #[cfg(unix)]
    {
        Command::new("sh")
            .arg("-c")
            .arg(&shell_line)
            .spawn()
            .map_err(|e| format!("could not run `{shell_line}`: {e}"))?;
    }
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", &shell_line])
            .spawn()
            .map_err(|e| format!("could not run `{shell_line}`: {e}"))?;
    }
    Ok(())
}

pub fn open_in_external_editor(app: &mut OtdApp, id: NodeId) -> Result<(), String> {
    let Some(body) = dat_body_for_export(app, id) else {
        return Err("only DAT operators can open in an external editor".into());
    };
    let path = edit_path(app, id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(&path, &body).map_err(|e| format!("{}: {e}", path.display()))?;
    spawn_editor(&app.settings.dat_external_editor, &path)?;
    let mtime = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    app.external_dat_edits
        .insert(id, ExternalDatWatch { path, mtime });
    app.document.status = format!(
        "Opened {} in external editor",
        app.graph.path(id).trim_start_matches('/')
    );
    Ok(())
}

pub fn poll_external_edits(app: &mut OtdApp) {
    let ids: Vec<NodeId> = app.external_dat_edits.keys().copied().collect();
    for id in ids {
        if !app.graph.contains(id) {
            app.external_dat_edits.remove(&id);
            continue;
        }
        let Some(watch) = app.external_dat_edits.get(&id).cloned() else {
            continue;
        };
        let Ok(meta) = std::fs::metadata(&watch.path) else {
            app.external_dat_edits.remove(&id);
            continue;
        };
        let Ok(mtime) = meta.modified() else {
            continue;
        };
        if mtime <= watch.mtime {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&watch.path) else {
            continue;
        };
        let key = content_param(app.graph.node(id).op_type.as_str());
        let current = app.graph.node(id).params[key].value.as_str();
        if current == text {
            app.external_dat_edits.insert(
                id,
                ExternalDatWatch {
                    path: watch.path,
                    mtime,
                },
            );
            continue;
        }
        app.edit("external editor");
        let _ = app.graph.set_param(id, key, Value::Str(text));
        app.document.history.end_gesture();
        app.external_dat_edits.insert(
            id,
            ExternalDatWatch {
                path: watch.path,
                mtime,
            },
        );
        app.document.status = format!(
            "Reloaded {} from external editor",
            app.graph.path(id).trim_start_matches('/')
        );
    }
}

#[cfg(test)]
#[path = "tests/dat_external__tests.rs"]
mod tests;
