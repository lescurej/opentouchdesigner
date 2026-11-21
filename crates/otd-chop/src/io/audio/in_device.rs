use super::super::*;
use crate::ops::ChopCtx;

use super::stream::{input_stream_config, rate_mismatch_note};

pub(crate) struct AudioIn {
    /// Kept alive so the callback keeps running; never read directly.
    _stream: cpal::Stream,
    ring: Arc<Mutex<Vec<f32>>>,
    /// Rate the project asked for. Compared on the next cook so a change
    /// reopens the stream.
    wanted_rate: f64,
    /// Rate the device actually opened at.
    sample_rate: f64,
    device: String,
}

const RING_CAP: usize = 1 << 16;

impl AudioIn {
    fn open(requested: &str, want_rate: f64) -> Result<AudioIn, String> {
        let host = cpal::default_host();
        let device = if requested.trim().is_empty() {
            host.default_input_device()
                .ok_or("no default audio input device")?
        } else {
            host.input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| device_name(d).map(|n| n == requested).unwrap_or(false))
                .ok_or_else(|| format!("no audio input device called `{requested}`"))?
        };
        let name = device_name(&device).unwrap_or_else(|| "?".to_string());
        let config = input_stream_config(&device, want_rate.round() as u32)
            .map_err(|e| format!("{name}: {e}"))?;
        let sample_rate = config.sample_rate() as f64;
        let channels = config.channels() as usize;

        let ring: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(RING_CAP)));
        let sink = ring.clone();
        let stream = device
            .build_input_stream(
                config.config(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Never block the real-time audio thread waiting on the
                    // cook — macOS will kill the process if this callback
                    // stalls. Drop a buffer instead of contending.
                    let Ok(mut buf) = sink.try_lock() else {
                        return;
                    };
                    for frame in data.chunks(channels.max(1)) {
                        let mono = frame.iter().sum::<f32>() / channels.max(1) as f32;
                        buf.push(mono);
                    }
                    if buf.len() > RING_CAP {
                        let excess = buf.len() - RING_CAP;
                        buf.drain(..excess);
                    }
                },
                move |err| log::warn!("audio input error: {err}"),
                None,
            )
            .map_err(|e| format!("{name}: {e}"))?;
        stream.play().map_err(|e| format!("{name}: {e}"))?;

        Ok(AudioIn {
            _stream: stream,
            ring,
            wanted_rate: want_rate,
            sample_rate,
            device: name,
        })
    }

    /// Take up to `want` samples. Underrun pads with the last value rather
    /// than with zeros, which clicks far less.
    fn take(&self, want: usize) -> Vec<f32> {
        let Ok(mut buf) = self.ring.lock() else {
            return vec![0.0; want];
        };
        if buf.len() >= want {
            let rest = buf.split_off(want);
            let out = std::mem::replace(&mut *buf, rest);
            return out;
        }
        let mut out = std::mem::take(&mut *buf);
        let pad = out.last().copied().unwrap_or(0.0);
        out.resize(want, pad);
        out
    }
}

pub(crate) fn params_audio_in() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "device".into(),
        Param::select("", &[("(default)", "")])
            .with_label("Device")
            .with_audio_input_devices(),
    );
    m.insert(
        "volume".into(),
        Param::float(1.0).with_label("Volume").with_range(0.0, 8.0),
    );
    m
}

pub(crate) fn cook_audio_in(c: &mut ChopCtx) -> ChopData {
    let requested = c
        .node
        .param("device")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    let volume = c
        .node
        .param("volume")
        .map(|p| p.eval(c.eval).as_f32())
        .unwrap_or(1.0);
    let path = c.path.to_string();
    let want_rate = otd_core::canonical_project_sample_rate(c.time.sample_rate);

    let needs_open = match c.io.audio.get(&path) {
        Some(a) => {
            let wrong_device = !requested.trim().is_empty() && a.device != requested;
            let wrong_rate = (a.wanted_rate - want_rate).abs() > 1.0;
            wrong_device || wrong_rate
        }
        None => true,
    };
    if needs_open {
        match AudioIn::open(&requested, want_rate) {
            Ok(a) => {
                match rate_mismatch_note(want_rate, a.sample_rate) {
                    Some(note) => c.io.note(&path, note),
                    None => c.io.clear_note(&path),
                }
                c.io.audio.insert(path.clone(), a);
            }
            Err(e) => {
                c.io.note(&path, e);
                c.io.audio.remove(&path);
            }
        }
    }

    let Some(audio) = c.io.audio.get(&path) else {
        // No device: a silent channel at control rate, so downstream
        // operators still have something with the right shape.
        let rate = c.rate();
        let n = slice_len(rate, c.time);
        return ChopData::new(vec![Channel::constant("chan1", 0.0, n)], rate, true);
    };
    let rate = audio.sample_rate;
    let n = slice_len(rate, c.time);
    let samples: Vec<f32> = audio.take(n).into_iter().map(|s| s * volume).collect();
    ChopData::new(vec![Channel::new("chan1", samples)], rate, true)
}
