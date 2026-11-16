use super::super::*;
use crate::ops::ChopCtx;

pub(crate) struct AudioFile {
    file: String,
    wav: crate::wav::Wav,
    /// Non-zero when ffmpeg produced this buffer. A PCM WAV is 0 and keeps
    /// the rate in its header, so a project-rate change does not reload it.
    decoded_at: f64,
}

pub(crate) fn params_audio_file() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "file".into(),
        Param::str("").with_label("File (.wav)").as_file_ref(),
    );
    m.insert(
        "play".into(),
        Param::menu("loop", &["loop", "once"]).with_label("Play"),
    );
    m.insert(
        "speed".into(),
        Param::float(1.0).with_label("Speed").with_range(-4.0, 4.0),
    );
    m.insert(
        "volume".into(),
        Param::float(1.0).with_label("Volume").with_range(0.0, 8.0),
    );
    m
}

/// Playback is a *function of the timeline*, not a private play head: the
/// sample at time `t` is always `wav[t * speed * rate]`. That is what makes
/// scrubbing the timeline scrub the audio, a loop range loop it, and a
/// headless render read the same samples the editor played.
pub(crate) fn cook_audio_file(c: &mut ChopCtx) -> ChopData {
    let get = |k: &str| c.node.param(k).map(|p| p.eval(c.eval));
    let file = get("file").map(|v| v.as_str()).unwrap_or_default();
    let looped = get("play").map(|v| v.as_str()).unwrap_or_default() != "once";
    let speed = get("speed").map(|v| v.as_f64()).unwrap_or(1.0);
    let volume = get("volume").map(|v| v.as_f32()).unwrap_or(1.0);
    let path = c.path.to_string();

    let silent = |c: &ChopCtx| {
        let rate = c.rate();
        let n = slice_len(rate, c.time);
        ChopData::new(vec![Channel::constant("chan1", 0.0, n)], rate, true)
    };
    if file.trim().is_empty() {
        c.io.files.remove(&path);
        c.io.clear_note(&path);
        return silent(c);
    }

    let project_rate = otd_core::canonical_project_sample_rate(c.time.sample_rate);
    let stale =
        c.io.files
            .get(&path)
            .map(|f| {
                f.file != file || (f.decoded_at > 0.0 && (f.decoded_at - project_rate).abs() > 1.0)
            })
            .unwrap_or(true);
    if stale {
        let resolved = match &c.io.base_dir {
            Some(dir) if !std::path::Path::new(&file).is_absolute() => dir.join(&file),
            _ => std::path::PathBuf::from(&file),
        };
        match crate::wav::load_for_rate(&resolved, project_rate) {
            Ok(prepared) => {
                c.io.clear_note(&path);
                c.io.files.insert(
                    path.clone(),
                    AudioFile {
                        file: file.clone(),
                        wav: prepared.wav,
                        decoded_at: prepared.decoded_at,
                    },
                );
            }
            Err(e) => {
                c.io.note(&path, e);
                c.io.files.remove(&path);
            }
        }
    }

    let Some(loaded) = c.io.files.get(&path) else {
        return silent(c);
    };
    let wav = &loaded.wav;
    if wav.samples.is_empty() {
        return silent(c);
    }
    let channels = wav.channels.max(1);
    let frames = wav.samples.len() / channels;
    if frames == 0 {
        return silent(c);
    }
    let len = frames as i64;
    let rate = wav.sample_rate;
    let n = slice_len(rate, c.time);
    let mut out_channels: Vec<Vec<f32>> = (0..channels).map(|_| Vec::with_capacity(n)).collect();
    for t in slice_times(c.time, n) {
        let sample_f = t * speed * rate;
        let frame = if looped {
            sample_f.rem_euclid(len as f64)
        } else if sample_f >= len as f64 {
            for ch in &mut out_channels {
                ch.push(0.0);
            }
            continue;
        } else {
            sample_f
        };
        if let Some(frame_samples) = wav.sample_at_frame(frame) {
            for (c, s) in out_channels.iter_mut().zip(frame_samples) {
                c.push(s * volume);
            }
        } else {
            for ch in &mut out_channels {
                ch.push(0.0);
            }
        }
    }
    let chop_channels: Vec<Channel> = out_channels
        .into_iter()
        .enumerate()
        .map(|(i, samples)| Channel::new(format!("chan{}", i + 1), samples))
        .collect();
    ChopData::new(chop_channels, rate, true)
}
