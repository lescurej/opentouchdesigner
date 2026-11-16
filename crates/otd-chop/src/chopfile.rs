use std::fs;
use std::io::Write;
use std::path::Path;

use crate::data::{CONTROL_RATE, Channel, ChopData};

pub const EXT: &str = "och";

pub fn resolve_path(base_dir: Option<&Path>, file: &str) -> std::path::PathBuf {
    let file = file.trim();
    if file.is_empty() {
        return Path::new("").to_path_buf();
    }
    if Path::new(file).is_absolute() {
        Path::new(file).to_path_buf()
    } else if let Some(dir) = base_dir {
        dir.join(file)
    } else {
        Path::new(file).to_path_buf()
    }
}

pub fn load(path: &Path) -> Result<ChopData, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
    parse(&text)
}

pub fn save(path: &Path, data: &ChopData) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("{path:?}: {e}"))?;
        }
    }
    let mut f = fs::File::create(path).map_err(|e| format!("{path:?}: {e}"))?;
    write!(f, "{}", serialize(data)).map_err(|e| e.to_string())
}

pub fn parse(text: &str) -> Result<ChopData, String> {
    let mut rate = CONTROL_RATE;
    let mut time_sliced = false;
    let mut channels = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("rate=") {
            rate = rest
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("bad rate `{rest}`"))?
                .max(1.0);
            continue;
        }
        if let Some(rest) = line.strip_prefix("timesliced=") {
            time_sliced = matches!(rest.trim(), "1" | "true" | "yes");
            continue;
        }
        let mut parts = line.split_whitespace();
        let name = parts
            .next()
            .ok_or_else(|| format!("empty channel line `{line}`"))?
            .to_string();
        let samples: Result<Vec<f32>, _> = parts.map(|s| s.parse()).collect();
        let samples = samples.map_err(|_| format!("bad sample in `{line}`"))?;
        if samples.is_empty() {
            return Err(format!("channel `{name}` has no samples"));
        }
        channels.push(Channel::new(name, samples));
    }
    if channels.is_empty() {
        return Err("no channels".into());
    }
    Ok(ChopData::new(channels, rate, time_sliced))
}

pub fn serialize(data: &ChopData) -> String {
    let mut out = format!(
        "# OpenTouchDesigner channel file\nrate={}\ntimesliced={}\n",
        data.sample_rate,
        if data.time_sliced { 1 } else { 0 }
    );
    for ch in &data.channels {
        out.push_str(&ch.name);
        for s in &ch.samples {
            out.push(' ');
            out.push_str(&format_sample(*s));
        }
        out.push('\n');
    }
    out
}

fn format_sample(v: f32) -> String {
    if v.is_nan() {
        "nan".into()
    } else if v.is_infinite() {
        if v.is_sign_positive() {
            "inf".into()
        } else {
            "-inf".into()
        }
    } else {
        format!("{v:.6}")
    }
}

#[cfg(test)]
#[path = "tests/chopfile__tests.rs"]
mod tests;
