use super::super::*;
use crate::ops::ChopCtx;

pub(crate) struct AudioFileRec {
    path: std::path::PathBuf,
    channels: usize,
    rate: f64,
    last_abs: f64,
    writer: crate::wav::WavOut,
}

pub(crate) fn params_audio_file_out() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "file".into(),
        Param::str("out.wav")
            .with_label("File (.wav)")
            .as_file_ref(),
    );
    m.insert("record".into(), Param::bool(false).with_label("Record"));
    m
}

fn interleave_channels(data: &ChopData) -> Vec<f32> {
    let n = data.num_samples();
    let mut out = Vec::with_capacity(n * data.num_channels());
    for i in 0..n {
        for ch in &data.channels {
            out.push(ch.samples.get(i).copied().unwrap_or(0.0));
        }
    }
    out
}

fn finish_audio_file(io: &mut Io, path: &str) {
    io.audio_file_out.remove(path);
}

/// Records the input to a WAV while Record is on, and passes it through.
///
/// A time-sliced input is appended once per cook. A whole buffer is the file:
/// writing it again each frame would repeat the same waveform.
pub(crate) fn cook_audio_file_out(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let record = c.b("record");
    let file = c.s("file");
    let op_path = c.path.to_string();
    let abs_time = c.time.abs_time;

    if !record {
        finish_audio_file(c.io, &op_path);
        c.io.clear_note(&op_path);
        return input;
    }
    if file.trim().is_empty() {
        finish_audio_file(c.io, &op_path);
        c.io.note(&op_path, "set a File to record to");
        return input;
    }
    if input.num_channels() == 0 {
        finish_audio_file(c.io, &op_path);
        c.io.note(&op_path, "nothing to record: the input has no channels");
        return input;
    }

    let resolved = crate::chopfile::resolve_path(c.io.base_dir.as_deref(), &file);
    let channels = input.num_channels();
    let rate = if input.sample_rate > 0.0 {
        input.sample_rate
    } else {
        c.rate()
    };

    if !input.time_sliced {
        finish_audio_file(c.io, &op_path);
        match crate::wav::write_file(&resolved, rate, channels, &interleave_channels(&input)) {
            Ok(()) => c.io.clear_note(&op_path),
            Err(e) => c.io.note(&op_path, e),
        }
        return input;
    }

    let same = c.io.audio_file_out.get(&op_path).is_some_and(|rec| {
        rec.path == resolved && rec.channels == channels && (rec.rate - rate).abs() < 1.0
    });
    if !same {
        finish_audio_file(c.io, &op_path);
        match crate::wav::WavOut::create(&resolved, channels, rate) {
            Ok(writer) => {
                c.io.audio_file_out.insert(
                    op_path.clone(),
                    AudioFileRec {
                        path: resolved,
                        channels,
                        rate,
                        last_abs: f64::NEG_INFINITY,
                        writer,
                    },
                );
            }
            Err(e) => {
                c.io.note(&op_path, e);
                return input;
            }
        }
    }

    let written = {
        let rec = c.io.audio_file_out.get_mut(&op_path).unwrap();
        if (rec.last_abs - abs_time).abs() < 1e-8 {
            return input;
        }
        rec.last_abs = abs_time;
        let samples = interleave_channels(&input);
        let write = rec.writer.write_frames(&samples);
        write.map(|()| rec.writer.frames())
    };
    match written {
        Ok(frames) => c.io.note(&op_path, format!("recording — {frames} samples")),
        Err(e) => c.io.note(&op_path, e),
    }
    input
}
