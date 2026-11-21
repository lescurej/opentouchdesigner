use super::super::*;
use crate::ops::ChopCtx;

use super::stream::{output_stream_config, rate_mismatch_note};

pub(crate) struct AudioOut {
    _stream: cpal::Stream,
    movie: Arc<Mutex<Option<Arc<Mutex<crate::ops::audiomovie::MovieTransport>>>>>,
    gain: Arc<std::sync::atomic::AtomicU32>,
    /// Rate the project asked for. A change reopens the stream.
    wanted_rate: f64,
    /// Rate the device actually opened at. Playback and resampling use this.
    sample_rate: f64,
    /// Channels the stream was opened with.
    channels: usize,
    /// Widest output the device offered, so we don't reopen every frame.
    device_max_channels: usize,
    device: String,
}

impl AudioOut {
    fn open(requested: &str, want_channels: usize, want_rate: f64) -> Result<AudioOut, String> {
        let host = cpal::default_host();
        let device = if requested.trim().is_empty() {
            host.default_output_device()
                .ok_or("no default audio output device")?
        } else {
            host.output_devices()
                .map_err(|e| e.to_string())?
                .find(|d| device_name(d).map(|n| n == requested).unwrap_or(false))
                .ok_or_else(|| format!("no audio output device called `{requested}`"))?
        };
        let name = device_name(&device).unwrap_or_else(|| "?".to_string());
        let (config, device_max_channels) =
            output_stream_config(&device, want_channels, want_rate.round() as u32)
                .map_err(|e| format!("{name}: {e}"))?;
        let sample_rate = config.sample_rate() as f64;
        let channels = config.channels() as usize;

        let movie: Arc<Mutex<Option<Arc<Mutex<crate::ops::audiomovie::MovieTransport>>>>> =
            Arc::new(Mutex::new(None));
        let movie_cb = movie.clone();
        let gain = Arc::new(std::sync::atomic::AtomicU32::new(1.0f32.to_bits()));
        let gain_cb = gain.clone();
        let stream = device
            .build_output_stream(
                config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let ch = channels.max(1);
                    match poll_movie(&movie_cb) {
                        MoviePoll::Ready(player) => {
                            if let Ok(mut transport) = player.try_lock() {
                                transport.render(sample_rate, data, ch);
                                let g = f32::from_bits(gain_cb.load(Ordering::Relaxed));
                                if (g - 1.0).abs() > 1e-6 {
                                    for s in data.iter_mut() {
                                        *s *= g;
                                    }
                                }
                            } else {
                                data.fill(0.0);
                            }
                        }
                        MoviePoll::Busy | MoviePoll::Idle => data.fill(0.0),
                    }
                },
                move |err| log::warn!("audio output error: {err}"),
                None,
            )
            .map_err(|e| format!("{name}: {e}"))?;
        stream.play().map_err(|e| format!("{name}: {e}"))?;

        Ok(AudioOut {
            _stream: stream,
            movie,
            gain,
            wanted_rate: want_rate,
            sample_rate,
            channels,
            device_max_channels,
            device: name,
        })
    }
}

enum MoviePoll {
    Idle,
    Busy,
    Ready(Arc<Mutex<crate::ops::audiomovie::MovieTransport>>),
}

fn poll_movie(
    slot: &Mutex<Option<Arc<Mutex<crate::ops::audiomovie::MovieTransport>>>>,
) -> MoviePoll {
    let Ok(guard) = slot.try_lock() else {
        return MoviePoll::Busy;
    };
    match guard.as_ref() {
        Some(player) => MoviePoll::Ready(Arc::clone(player)),
        None => MoviePoll::Idle,
    }
}

pub(crate) fn params_audio_out() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "device".into(),
        Param::select("", &[("(default)", "")])
            .with_label("Device")
            .with_audio_output_devices(),
    );
    m.insert(
        "volume".into(),
        Param::float(1.0).with_label("Volume").with_range(0.0, 2.0),
    );
    m.insert("active".into(), Param::bool(true).with_label("Active"));
    m
}

pub(crate) fn cook_audio_out(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let get = |k: &str| c.node.param(k).map(|p| p.eval(c.eval));
    let requested = get("device").map(|v| v.as_str()).unwrap_or_default();
    let volume = get("volume").map(|v| v.as_f32()).unwrap_or(1.0);
    let active = get("active").map(|v| v.as_bool()).unwrap_or(true);
    let path = c.path.to_string();
    let want_rate = otd_core::canonical_project_sample_rate(c.time.sample_rate);

    if c.io.rt_audio_armed() {
        if !active {
            c.io.note(&path, "audio output inactive");
        } else {
            c.io.clear_note(&path);
        }
        return input;
    }

    if !active || input.num_channels() == 0 {
        if let Some(out) = c.io.audio_out.get(&path) {
            if let Ok(mut slot) = out.movie.lock() {
                if let Some(prev) = slot.take() {
                    if let Ok(mut transport) = prev.lock() {
                        transport.set_device_driven(false);
                    }
                }
            }
        }
        return input;
    }

    let movie_in = c
        .node
        .inputs
        .first()
        .copied()
        .flatten()
        .and_then(|id| crate::ops::audiomovie::transport_for_input(c.graph, Some(id), c.io));
    let input_channels = input.num_channels().max(1);
    let want_channels = movie_in
        .as_ref()
        .and_then(|player| player.lock().ok().map(|t| t.channel_count()))
        .unwrap_or(input_channels);
    let needs_open = match c.io.audio_out.get(&path) {
        Some(a) => {
            let wrong_device = !requested.trim().is_empty() && a.device != requested;
            let too_narrow = want_channels > a.channels && a.channels < a.device_max_channels;
            let wrong_rate = (a.wanted_rate - want_rate).abs() > 1.0;
            wrong_device || too_narrow || wrong_rate
        }
        None => true,
    };
    if movie_in.is_some() && needs_open {
        match AudioOut::open(&requested, want_channels, want_rate) {
            Ok(a) => {
                match rate_mismatch_note(want_rate, a.sample_rate) {
                    Some(note) => c.io.note(&path, note),
                    None => c.io.clear_note(&path),
                }
                c.io.audio_out.insert(path.clone(), a);
            }
            Err(e) => {
                c.io.note(&path, e);
                c.io.audio_out.remove(&path);
            }
        }
    }

    if let Some(out) = c.io.audio_out.get(&path) {
        out.gain
            .store(volume.to_bits(), std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut slot) = out.movie.lock() {
            if let Some(player) = movie_in.clone() {
                if let Ok(mut transport) = player.lock() {
                    transport.set_device_driven(true);
                }
                *slot = Some(player);
            } else if let Some(prev) = slot.take() {
                if let Ok(mut transport) = prev.lock() {
                    transport.set_device_driven(false);
                }
            }
        }
    } else if !movie_in.is_some() {
        c.io.note(
            &path,
            "no realtime audio plan (check rt chain and bridges from control CHOPs)",
        );
    }
    input
}
