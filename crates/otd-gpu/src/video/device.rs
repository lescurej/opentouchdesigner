//! Camera capture: mode negotiation and device selection.

use std::process::Stdio;

use super::decode::ffmpeg_command;

/// The capture input ffmpeg uses for a camera on this platform.
pub fn camera_input() -> (&'static str, &'static str) {
    if cfg!(target_os = "macos") {
        ("avfoundation", "0")
    } else if cfg!(target_os = "windows") {
        ("dshow", "video=default")
    } else {
        ("v4l2", "/dev/video0")
    }
}

/// One thing a camera can actually do.
#[derive(Clone, Debug, PartialEq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub rates: Vec<f64>,
}

/// A line of ffmpeg's supported-mode list: `  1280x720@[15.000000 30.000000]fps`,
/// usually behind a `[in#0 @ 0x7f…]` prefix.
pub(crate) fn parse_mode_line(line: &str) -> Option<Mode> {
    // Drop any bracketed logging prefix without disturbing the `@[...]` that
    // carries the rates.
    let body = match line.rfind("] ") {
        Some(i) if line.starts_with('[') => &line[i + 2..],
        _ => line,
    };
    let (size, rest) = body.trim().split_once('@')?;
    let (w, h) = size.trim().split_once('x')?;
    let width = w.trim().parse().ok()?;
    let height = h.trim().parse().ok()?;
    let rates: Vec<f64> = rest
        .trim_start_matches('[')
        .split(']')
        .next()?
        .split_whitespace()
        .filter_map(|r| r.parse().ok())
        .collect();
    (width > 0 && height > 0).then_some(Mode {
        width,
        height,
        rates,
    })
}

/// Ask a capture device what it can do.
///
/// There is no query for this, so the question is asked by requesting
/// something impossible: ffmpeg refuses and prints the list. It costs a third
/// of a second and no camera warm-up, which is far cheaper than opening the
/// device several times to find out by trial.
pub fn camera_modes(format: &str, device: &str) -> Vec<Mode> {
    let Some(mut ffmpeg) = ffmpeg_command() else {
        return Vec::new();
    };
    let out = ffmpeg
        .args(["-hide_banner", "-v", "error", "-f", format])
        .args(["-video_size", "1x1", "-framerate", "1"])
        .args(["-i", device, "-frames:v", "1", "-f", "null", "-"])
        .stdin(Stdio::null())
        .output();
    let Ok(out) = out else { return Vec::new() };
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(parse_mode_line)
        .collect()
}

/// The mode closest to what the patch asked for, and a rate the device has.
///
/// "Closest" is by pixel count rather than by width: asking for 1280×720 on a
/// portrait camera should not land on 1080×1920 just because a width matches.
pub(crate) fn choose_mode(modes: &[Mode], want_w: u32, want_h: u32, want_fps: f64) -> Option<(Mode, f64)> {
    let want_pixels = (want_w as i64) * (want_h as i64);
    let best = modes.iter().min_by_key(|m| {
        let pixels = (m.width as i64) * (m.height as i64);
        (pixels - want_pixels).abs()
    })?;
    let rate = best
        .rates
        .iter()
        .copied()
        .min_by(|a, b| {
            (a - want_fps)
                .abs()
                .partial_cmp(&(b - want_fps).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(want_fps);
    Some((best.clone(), rate))
}
