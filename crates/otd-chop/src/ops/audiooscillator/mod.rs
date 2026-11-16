use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{Channel, ChopData, slice_len};
use crate::ops::ChopCtx;
use crate::ops::helpers::with_rate;

use super::{spec_animated_rt, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    let mut m = with_rate(params! {
        "type" => Param::menu("sine", &["sine", "triangle", "ramp", "square", "pulse"])
            .with_label("Type"),
        "frequency" => Param::float(440.0)
            .with_label("Base Frequency (Hz)")
            .with_range(0.0, 20000.0)
            // 20 Hz is the low end of the sweep; 0 stays at the slider's end.
            .with_logarithmic(20.0),
        // 1 keeps the pitch input in octaves. 12 is semitones, 1200 is cents.
        "unitsperoctave" => Param::float(1.0).with_label("Units per Octave").with_range(0.01, 1200.0),
        "amplitude" => Param::float(0.5).with_label("Amplitude").with_range(0.0, 1.0),
        "phase" => Param::float(0.0).with_label("Phase").with_range(0.0, 1.0),
        "pulsewidth" => Param::float(0.5).with_label("Pulse Width").with_range(0.0, 1.0),
        // Off holds a control-rate pitch for the frame, then steps. On glides
        // from the previous frame's pitch to this one across the audio slice.
        "smooth" => Param::bool(false).with_label("Smooth Pitch Changes"),
        "name" => Param::str("chan1").with_label("Channel Name"),
    });
    // Device rate, not the 60 Hz control rate. The speaker callback renders
    // from these parameters directly; a 60 Hz buffer resampled up to 48 kHz
    // kinks at every cook boundary.
    m.insert(
        "rate".into(),
        Param::float(48_000.0)
            .with_label("Sample Rate")
            .with_range(8_000.0, 48_000.0),
    );
    m
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate().max(8000.0);
    let n = slice_len(rate, c.time);
    let kind = c.menu("type");
    let base_freq = c.f("frequency");
    let units_per_octave = c.f("unitsperoctave");
    let amp = c.f("amplitude");
    let base_phase = c.f("phase");
    let pw = c.f("pulsewidth");
    let smooth = c.b("smooth");

    let pitch_in = c.input(0).clone();
    let reset_in = c.input(1).clone();
    let playback_in = c.input(2).clone();
    let use_playback = playback_in.num_channels() > 0
        && playback_in.channels.iter().any(|ch| !ch.samples.is_empty());

    let num_out = pitch_in.num_channels().max(1);
    c.state.fit(num_out);

    let reset_chan = reset_in.channels.first().map(|ch| ch.samples.as_slice());
    let mut channels = Vec::with_capacity(num_out);
    for ci in 0..num_out {
        let pb_idx = ci.min(playback_in.num_channels().saturating_sub(1));
        let playback = use_playback.then(|| {
            (
                playback_in.channels[pb_idx].samples.as_slice(),
                playback_in.sample_rate.max(1.0),
            )
        });
        let pitch_samples = pitch_in
            .channels
            .get(ci)
            .map(|ch| ch.samples.as_slice())
            .unwrap_or(&[]);
        let mut osc = crate::rt::OscillatorState::load(
            c.state.f[ci],
            c.state.prev[ci],
            c.state.i[ci] != 0,
        );
        let mut samples = vec![0.0; n];
        osc.process_block(
            &mut samples,
            kind,
            base_freq,
            units_per_octave,
            amp,
            base_phase,
            pw,
            smooth,
            rate,
            pitch_samples,
            reset_chan,
            playback,
        );
        let (phase, prev, init) = osc.store();
        c.state.f[ci] = phase;
        c.state.prev[ci] = prev;
        c.state.i[ci] = if init { 1 } else { 0 };

        let name = output_channel_name(c, &pitch_in, ci, num_out);
        channels.push(Channel::new(name, samples));
    }
    ChopData::new(channels, rate, true)
}

fn output_channel_name(c: &ChopCtx<'_>, pitch_in: &ChopData, ci: usize, num_out: usize) -> String {
    if num_out > 1 {
        pitch_in
            .channels
            .get(ci)
            .map(|ch| ch.name.clone())
            .unwrap_or_else(|| format!("chan{}", ci + 1))
    } else {
        c.s("name")
    }
}

pub const TYPE_NAME: &str = "audiooscillatorCHOP";

pub fn spec() -> ChopSpec {
    spec_animated_rt(
        TYPE_NAME,
        "Audio Oscillator",
        &["pitch", "reset", "playback"],
        "Sine, triangle, ramp, square or pulse. Pitch, reset and playback inputs override the parameters.",
        params,
        cook,
    )
}
