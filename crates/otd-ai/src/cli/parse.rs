use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::provider::Provider;

/// Claude Code's stream-json output is one object per line, ending in a
/// `result`. That last object is exactly what `--output-format json` would
/// have returned on its own — the reply in `result`, an error the same field
/// with `is_error` set — so the parsing below is the same either way, and
/// only the finding of it differs.
pub(crate) fn parse_claude(out: &str, err: &str) -> Result<String, String> {
    let last = out
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .rfind(|event| event.get("type").and_then(|t| t.as_str()) == Some("result"));

    let Some(body) = last else {
        return Err(format!(
            "Claude Code did not return JSON: {}",
            tail(out.trim())
        ));
    };

    let text = body
        .get("result")
        .and_then(|r| r.as_str())
        .unwrap_or_default();

    if body.get("is_error").and_then(|e| e.as_bool()) == Some(true) {
        return Err(explain(text, err, body.get("subtype")));
    }
    if text.trim().is_empty() {
        let reason = body
            .get("stop_reason")
            .and_then(|s| s.as_str())
            .unwrap_or("no reason given");
        return Err(format!("Claude Code returned nothing (stopped: {reason})"));
    }
    Ok(text.to_string())
}

/// `codex exec --json` streams one JSON object per line. The reply is the
/// last completed `agent_message`; a failure arrives as a top-level `error`
/// or a `turn.failed`, both of which can appear *after* useful lines, so the
/// whole stream is read before deciding.
pub(crate) fn parse_codex(out: &str, err: &str) -> Result<String, String> {
    let mut answer: Option<String> = None;
    let mut failure: Option<String> = None;

    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("item.completed") => {
                let item = event.get("item");
                let kind = item
                    .and_then(|i| i.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or_default();
                if kind == "agent_message" {
                    if let Some(text) = item.and_then(|i| i.get("text")).and_then(|t| t.as_str()) {
                        answer = Some(text.to_string());
                    }
                }
                // An `error` item is a warning in this stream — a model
                // metadata note, a truncated skill list — and the turn
                // usually completes anyway. Only a fatal one is recorded.
            }
            Some("error") | Some("turn.failed") => {
                let message = event
                    .get("message")
                    .or_else(|| event.get("error").and_then(|e| e.get("message")))
                    .and_then(|m| m.as_str())
                    .unwrap_or("the turn failed");
                failure = Some(unwrap_nested(message));
            }
            _ => {}
        }
    }

    match answer {
        // An answer that arrived is worth more than a late warning.
        Some(text) if !text.trim().is_empty() => Ok(text),
        _ => Err(format!(
            "Codex: {}",
            failure.unwrap_or_else(|| {
                let detail = tail(err);
                match detail.is_empty() {
                    true => "the turn produced no reply".into(),
                    false => detail,
                }
            })
        )),
    }
}

/// Codex wraps the upstream error body in its own message as a JSON string.
/// One level of that is worth unwrapping, because the sentence inside is the
/// one a person can act on — "not supported when using Codex with a ChatGPT
/// account" rather than a brace-laden quotation of it.
fn unwrap_nested(message: &str) -> String {
    let trimmed = message.trim();
    if !trimmed.starts_with('{') {
        return trimmed.to_string();
    }
    serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message"))
                .or_else(|| v.get("message"))
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| trimmed.to_string())
}

/// Say what went wrong and, where the CLI has a known failure mode, what to
/// do about it. A subscription that has run out for the week is the common
/// one and it does not read as anything in particular on its own.
fn explain(text: &str, err: &str, subtype: Option<&serde_json::Value>) -> String {
    let detail = match text.trim().is_empty() {
        false => text.trim().to_string(),
        true => tail(err),
    };
    let kind = subtype.and_then(|s| s.as_str()).unwrap_or_default();
    let advice = if detail.contains("login") || detail.contains("authenticat") || kind == "auth" {
        Some("run `claude` once in a terminal and sign in")
    } else if detail.contains("limit") || detail.contains("quota") {
        Some("the subscription's limit — wait for the reset, or use an API key provider")
    } else {
        None
    };
    match advice {
        Some(advice) if !detail.is_empty() => format!("Claude Code: {detail} — {advice}"),
        Some(advice) => format!("Claude Code: {advice}"),
        None if !detail.is_empty() => format!("Claude Code: {detail}"),
        None => "Claude Code failed without saying why".into(),
    }
}

/// The last few lines of a stream, for an error message. The end is where the
/// reason is; the beginning is startup noise.
pub(crate) fn tail(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("; ")
        .chars()
        .take(300)
        .collect()
}

pub(crate) fn not_found(provider: Provider) -> String {
    format!(
        "{} is not installed on this machine — install it and sign in, or pick a provider that takes an API key",
        provider.label()
    )
}

/// The command a provider runs, if it can be found.
pub(crate) fn command_name(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::ClaudeCode => Some("claude"),
        Provider::Codex => Some("codex"),
        _ => None,
    }
}

/// The environment variable that overrides the search, for a machine where
/// the binary is somewhere nobody would guess.
pub(crate) fn override_var(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::ClaudeCode => Some("OTD_CLAUDE_BIN"),
        Provider::Codex => Some("OTD_CODEX_BIN"),
        _ => None,
    }
}

/// Find the provider's binary.
///
/// `PATH` first, then the handful of places these two actually install to.
/// The fallback list is not belt and braces: a GUI application launched from
/// Finder or the Start menu gets a `PATH` that contains none of them, so for
/// most users the fallback *is* the lookup.
pub fn binary(provider: Provider) -> Option<PathBuf> {
    let name = command_name(provider)?;

    if let Some(var) = override_var(provider) {
        if let Some(value) = std::env::var_os(var) {
            let path = PathBuf::from(value);
            if is_executable(&path) {
                return Some(path);
            }
        }
    }

    for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        if let Some(hit) = probe(&dir, name) {
            return Some(hit);
        }
    }

    // Order matters, and not in the way you would guess. Both CLIs' own
    // installers put them in `~/.local/bin` and keep them updated there; a
    // copy in `/opt/homebrew/bin` or `/usr/local/bin` is usually an older
    // install somebody has forgotten about. Preferring the system directories
    // finds a version from months ago on a machine that has a current one
    // three lines further down the list — so the per-user paths go first.
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
    {
        candidates.extend([
            home.join(".local/bin"),
            home.join(".claude/local"),
            home.join(".bun/bin"),
            home.join(".volta/bin"),
            home.join(".npm-global/bin"),
            home.join("AppData/Roaming/npm"),
        ]);
    }
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    candidates.into_iter().find_map(|dir| probe(&dir, name))
}

/// Look for `name` in `dir`, including the extensions Windows needs.
fn probe(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    let plain = dir.join(name);
    if is_executable(&plain) {
        return Some(plain);
    }
    if cfg!(windows) {
        for ext in ["cmd", "exe", "bat"] {
            let with_ext = dir.join(format!("{name}.{ext}"));
            if is_executable(&with_ext) {
                return Some(with_ext);
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &std::path::Path) -> bool {
    path.is_file()
}

/// Whether the CLI is there, and which version — the equivalent of "has a
/// key" for a provider that does not take one.
///
/// Spawns a process, so call it when the settings window opens rather than
/// once a frame.
pub fn detect(provider: Provider) -> Result<String, String> {
    let bin = binary(provider).ok_or_else(|| not_found(provider))?;
    let out = Command::new(&bin)
        .arg("--version")
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run {}: {e}", bin.display()))?;
    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    match version.is_empty() {
        true => Ok(bin.display().to_string()),
        false => Ok(version),
    }
}
