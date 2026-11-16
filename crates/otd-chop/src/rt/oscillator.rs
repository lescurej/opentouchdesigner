use crate::ops::wave;

fn hz_from_pitch(base: f32, pitch: f32, units_per_octave: f32) -> f32 {
    let u = units_per_octave.max(1e-6);
    if !base.is_finite() || !pitch.is_finite() {
        return if base.is_finite() { base } else { 440.0 };
    }
    base * 2.0f32.powf(pitch / u)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OscillatorState {
    pub phase: f32,
    pub prev_pitch: f32,
    pub pitch_init: bool,
}

impl OscillatorState {
    pub fn load(phase: f32, prev_pitch: f32, pitch_init: bool) -> Self {
        Self {
            phase: if phase.is_finite() { phase } else { 0.0 },
            prev_pitch,
            pitch_init,
        }
    }

    pub fn store(&self) -> (f32, f32, bool) {
        (self.phase, self.prev_pitch, self.pitch_init)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_block(
        &mut self,
        out: &mut [f32],
        kind: usize,
        base_freq: f32,
        units_per_octave: f32,
        amp: f32,
        base_phase: f32,
        pw: f32,
        smooth: bool,
        sample_rate: f64,
        pitch: &[f32],
        reset: Option<&[f32]>,
        playback: Option<(&[f32], f64)>,
    ) {
        let n = out.len();
        if n == 0 {
            return;
        }
        let flat = pitch_is_flat(pitch);
        let end_pitch = pitch.last().copied().unwrap_or(0.0);
        let mut from_pitch = self.prev_pitch;
        if !self.pitch_init || !from_pitch.is_finite() {
            from_pitch = end_pitch;
        }
        for (si, slot) in out.iter_mut().enumerate() {
            if reset.is_some_and(|r| sample_at(r, si) >= 0.5) {
                self.phase = 0.0;
                *slot = 0.0;
                continue;
            }
            let pitch_v = if smooth && flat {
                glide(from_pitch, end_pitch, si, n)
            } else {
                sample_at(pitch, si)
            };
            let freq = hz_from_pitch(base_freq, pitch_v, units_per_octave);
            let v = if let Some((buf, pb_rate)) = playback {
                let s = sample_wrapped(buf, self.phase);
                self.phase = advance_playback(self.phase, freq / pb_rate as f32, buf.len());
                s
            } else {
                let v = wave(kind, self.phase + base_phase, pw);
                self.phase = advance_cycles(self.phase, freq / sample_rate as f32);
                v
            };
            let sample = v * amp;
            *slot = if sample.is_finite() { sample } else { 0.0 };
        }
        self.prev_pitch = end_pitch;
        self.pitch_init = true;
    }
}

fn pitch_is_flat(samples: &[f32]) -> bool {
    let Some(first) = samples.first() else {
        return true;
    };
    samples.iter().all(|s| (*s - first).abs() <= 1e-4)
}

fn glide(from: f32, to: f32, si: usize, n: usize) -> f32 {
    if n <= 1 {
        return to;
    }
    let t = (si + 1) as f32 / n as f32;
    let pitch = from + (to - from) * t;
    if pitch.is_finite() { pitch } else { to }
}

fn sample_at(samples: &[f32], si: usize) -> f32 {
    samples
        .get(si)
        .or_else(|| samples.last())
        .copied()
        .unwrap_or(0.0)
}

pub(crate) fn advance_cycles(phase: f32, step: f32) -> f32 {
    let phase = if phase.is_finite() { phase } else { 0.0 };
    if !step.is_finite() {
        let wrapped = phase.rem_euclid(1.0);
        return if wrapped.is_finite() { wrapped } else { 0.0 };
    }
    let next = (phase + step).rem_euclid(1.0);
    if next.is_finite() { next } else { 0.0 }
}

pub(crate) fn advance_playback(pos: f32, step: f32, len: usize) -> f32 {
    let len = len as f32;
    if len <= 0.0 {
        return 0.0;
    }
    let pos = if pos.is_finite() { pos } else { 0.0 };
    let step = if step.is_finite() { step } else { 0.0 };
    let next = (pos + step).rem_euclid(len);
    if next.is_finite() { next } else { 0.0 }
}

pub(crate) fn sample_wrapped(buf: &[f32], pos: f32) -> f32 {
    if buf.is_empty() || !pos.is_finite() {
        return 0.0;
    }
    let len = buf.len() as f32;
    let pos = pos.rem_euclid(len);
    if !pos.is_finite() {
        return 0.0;
    }
    let lo = pos.floor() as usize;
    let hi = (lo + 1) % buf.len();
    let frac = pos - lo as f32;
    buf[lo] * (1.0 - frac) + buf[hi] * frac
}

#[cfg(test)]
#[path = "../tests/rt_oscillator__tests.rs"]
mod tests;
