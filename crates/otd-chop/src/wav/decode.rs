use std::path::Path;
use std::process::{Command, Stdio};

use super::io::parse;
use super::types::{DECODED_RATE, PreparedWav, Wav};

pub fn load(path: &Path) -> Result<Wav, String> {
    load_for_rate(path, DECODED_RATE).map(|prepared| prepared.wav)
}

/// PCM WAV stays at its header rate. Anything else is decoded at `decode_rate`.
pub fn load_for_rate(path: &Path, decode_rate: f64) -> Result<PreparedWav, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    match parse(&bytes) {
        Ok(wav) => Ok(PreparedWav {
            wav,
            decoded_at: 0.0,
        }),
        Err(why) => decode(path, decode_rate)
            .map(|wav| PreparedWav {
                decoded_at: wav.sample_rate,
                wav,
            })
            .map_err(|e| match e {
                DecodeError::NoFfmpeg(missing) => format!("{why} — {missing}"),
                DecodeError::Failed(e) => e,
            }),
    }
}

enum DecodeError {
    NoFfmpeg(String),
    Failed(String),
}

fn why_it_failed(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let first = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    let detail = match first.starts_with('[') {
        true => first
            .split_once("] ")
            .map(|(_, rest)| rest)
            .unwrap_or(first),
        false => first,
    };
    if detail.contains("does not contain any stream") {
        return "no audio track in that file".to_string();
    }
    match detail.is_empty() {
        true => "ffmpeg could not decode it".to_string(),
        false => detail.to_string(),
    }
}

/// ffmpeg interleaved order for the layouts we play back: left, right, center,
/// LFE, back left, back right, side left, side right.
fn pcm_layout(channels: usize) -> Option<&'static str> {
    match channels {
        1 => Some("mono"),
        2 => Some("stereo"),
        6 => Some("5.1"),
        8 => Some("7.1"),
        _ => None,
    }
}

fn probe_channels(path: &Path) -> Result<usize, DecodeError> {
    let Some(ffprobe) = otd_core::tools::ffprobe() else {
        return Ok(1);
    };
    let out = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=channels",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| DecodeError::Failed(format!("could not run ffprobe: {e}")))?;
    if !out.status.success() {
        return Ok(1);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let n = text.trim().parse::<usize>().unwrap_or(1);
    Ok(n.max(1))
}

fn decode(path: &Path, decode_rate: f64) -> Result<Wav, DecodeError> {
    let Some(ffmpeg) = otd_core::tools::ffmpeg() else {
        return Err(DecodeError::NoFfmpeg(otd_core::tools::missing_ffmpeg()));
    };
    let channels = probe_channels(path)?;
    // Ask for the real channel count and a named layout. Without that, ffmpeg's
    // raw output defaults toward stereo and a 5.1 / 7.1 track loses the rest.
    let channel_count = channels.to_string();
    let decode_rate = if decode_rate.is_finite() {
        decode_rate.clamp(8_000.0, 192_000.0)
    } else {
        DECODED_RATE
    };
    let rate = format!("{decode_rate:.0}");
    let path_arg = path.to_string_lossy().into_owned();
    let mut args = vec![
        "-v",
        "error",
        "-i",
        path_arg.as_str(),
        "-vn",
        "-ac",
        channel_count.as_str(),
    ];
    let layout_storage;
    if let Some(layout) = pcm_layout(channels) {
        layout_storage = layout.to_string();
        args.extend(["-channel_layout", &layout_storage]);
    }
    args.extend(["-ar", &rate, "-f", "s16le", "-"]);
    let out = Command::new(ffmpeg)
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| DecodeError::Failed(format!("could not run ffmpeg: {e}")))?;

    if !out.status.success() {
        return Err(DecodeError::Failed(why_it_failed(&out.stderr)));
    }
    if out.stdout.is_empty() {
        return Err(DecodeError::Failed(
            "no audio track in that file".to_string(),
        ));
    }
    let samples: Vec<f32> = out
        .stdout
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect();
    let frame_count = samples.len() / channels;
    Ok(Wav {
        sample_rate: decode_rate,
        channels,
        samples: samples[..frame_count * channels].to_vec(),
    })
}
