//! File probing, still decode, and ffmpeg tool discovery.

use std::path::Path;
use std::process::Command;

#[cfg(target_os = "macos")]
use super::macos;

use super::Frame;

/// Extensions decoded in-process, without ffmpeg.
const STILL_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tga", "tif", "tiff", "ico", "ppm", "pnm",
];

pub fn is_still(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| STILL_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn position_from_units(value: f64, unit: &str, fps: f64, span: i64) -> i64 {
    match unit {
        "seconds" => (value * fps.max(1e-6)).floor() as i64,
        "frames" | "indices" => value.floor() as i64,
        "fraction" => {
            if span <= 1 {
                0
            } else {
                (value.clamp(0.0, 1.0) * (span - 1) as f64).round() as i64
            }
        }
        _ => value.floor() as i64,
    }
}

/// What a source is worth knowing before the first frame arrives.
#[derive(Clone, Debug, Default)]
pub struct Info {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
}

// -------------------------------------------------------- finding the tools
//
// The lookup itself lives in `otd_core::tools`: the movie node, the audio
// file CHOP and the assistant's CLIs all have the same problem with a
// Finder-launched `PATH`, and it was worth solving once.

pub(crate) fn ffmpeg_command() -> Option<Command> {
    otd_core::tools::ffmpeg().map(Command::new)
}

pub(crate) fn ffprobe_command() -> Option<Command> {
    otd_core::tools::ffprobe().map(Command::new)
}

/// Whether both tools were found. Lets a caller say "ffmpeg is missing"
/// rather than guessing at why a file would not open.
pub fn tools_installed() -> bool {
    otd_core::tools::media_tools_installed()
}

/// One wording for the one thing the user has to do about it.
pub fn missing_ffmpeg() -> String {
    otd_core::tools::missing_ffmpeg()
}

// ------------------------------------------------------------------ probing

/// Ask ffprobe what a file is. Returns `None` when ffprobe is missing or the
/// file is not media — the caller turns that into a message on the node.
pub fn probe(path: &str) -> Option<Info> {
    #[cfg(target_os = "macos")]
    if let Some(info) = macos::probe(path) {
        return Some(info);
    }
    let out = ffprobe_command()?
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate:format=duration",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let stream = json.get("streams")?.get(0)?;
    let width = stream.get("width")?.as_u64()? as u32;
    let height = stream.get("height")?.as_u64()? as u32;
    let fps = stream
        .get("r_frame_rate")
        .and_then(|v| v.as_str())
        .and_then(parse_rational)
        .filter(|f| *f > 0.0)
        .unwrap_or(30.0);
    let duration = json
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);
    Some(Info {
        width,
        height,
        fps,
        duration,
    })
}

/// ffprobe reports frame rates as `30000/1001`, not as a decimal.
pub(crate) fn parse_rational(text: &str) -> Option<f64> {
    match text.split_once('/') {
        Some((n, d)) => {
            let (n, d) = (n.trim().parse::<f64>().ok()?, d.trim().parse::<f64>().ok()?);
            (d != 0.0).then_some(n / d)
        }
        None => text.trim().parse().ok(),
    }
}
/// Decode a still image to RGBA8.
pub(crate) fn decode_still(path: &Path) -> Result<Frame, String> {
    let image = image::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let rgba = image.to_rgba8();
    Ok(Frame {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}
