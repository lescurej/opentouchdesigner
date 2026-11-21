use super::smpte::{
    SYNC, Smpte, decode_ltc_frame, encode_ltc_frame_user, labels_to_total, smpte_from_drop_frames,
    smpte_non_drop_frames,
};

fn bmc_sign(bits: &[u8; 80], bit_index: usize, second_half: bool) -> f32 {
    let mut flips = 1usize;
    for bit in bits.iter().take(bit_index) {
        flips += 1 + *bit as usize;
    }
    if second_half && bits[bit_index] == 1 {
        flips += 1;
    }
    if flips % 2 == 0 { -1.0 } else { 1.0 }
}

pub struct LtcStream {
    pub fps: f64,
    pub drop_frame: bool,
    pub level: f32,
    pub duplicate_high_fps: bool,
    pub user: [u8; 8],
}

#[cfg(test)]
pub fn ltc_samples_from_index(
    start: i64,
    n: usize,
    rate: f64,
    fps: f64,
    drop_frame: bool,
    level: f32,
) -> Vec<f32> {
    let rate = rate.max(1.0);
    let fps = fps.max(1.0);
    let stream = LtcStream {
        fps,
        drop_frame,
        level,
        duplicate_high_fps: false,
        user: [0; 8],
    };
    let step = fps / rate;
    let mut out = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let t = (start + i as i64) as f64 / rate;
        if t < 0.0 {
            out.push(0.0);
            i += 1;
            continue;
        }
        out.extend(ltc_samples_from_clock(t * fps, n - i, step, false, &stream));
        break;
    }
    out
}

pub fn ltc_samples_from_clock(
    frame_pos: f64,
    n: usize,
    step: f64,
    hold: bool,
    stream: &LtcStream,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(n);
    let mut cached = i64::MIN;
    let mut bits = [0u8; 80];
    let base = if hold { frame_pos.floor() } else { 0.0 };
    let frac0 = if hold { frame_pos.fract() } else { frame_pos };
    for i in 0..n {
        let pos = if hold {
            base + (frac0 + i as f64 * step).fract()
        } else {
            frame_pos + i as f64 * step + 1e-9
        };
        if pos < 0.0 {
            out.push(0.0);
            continue;
        }
        let frame_index = pos.floor() as i64;
        let frac = (pos - frame_index as f64).clamp(0.0, 0.999999);
        let (shown, encode_fps) = shown_frame(frame_index, stream);
        if shown != cached {
            let smpte = if stream.drop_frame {
                smpte_from_drop_frames(shown)
            } else {
                smpte_non_drop_frames(shown, encode_fps)
            };
            bits = encode_ltc_frame_user(&smpte, stream.drop_frame, encode_fps, &stream.user);
            cached = shown;
        }
        let bit_pos = frac * 80.0;
        let bit_index = (bit_pos.floor() as usize).min(79);
        let bit_frac = bit_pos - bit_index as f64;
        out.push(bmc_sign(&bits, bit_index, bit_frac >= 0.5) * stream.level);
    }
    out
}

fn shown_frame(frame_index: i64, stream: &LtcStream) -> (i64, f64) {
    let fps = stream.fps.max(1.0);
    if stream.duplicate_high_fps && fps > 30.5 && !stream.drop_frame {
        let ratio = (fps / 30.0).round().max(1.0) as i64;
        (frame_index / ratio.max(1), 30.0)
    } else {
        (frame_index, fps)
    }
}

fn transitions(samples: &[f32]) -> Vec<usize> {
    if samples.is_empty() {
        return Vec::new();
    }
    let mean = samples.iter().copied().sum::<f32>() / samples.len() as f32;
    let peak = samples.iter().fold(0.0f32, |m, s| m.max((s - mean).abs()));
    if peak < 1.0e-4 {
        return Vec::new();
    }
    let hys = peak * 0.25;
    let mut edges = Vec::new();
    let mut state = 0i8;
    for (i, s) in samples.iter().enumerate() {
        let v = *s - mean;
        let next = if v > hys {
            1
        } else if v < -hys {
            -1
        } else {
            state
        };
        if state != 0 && next != 0 && next != state {
            edges.push(i);
        }
        if next != 0 {
            state = next;
        }
    }
    edges
}

fn push_bit(reg: &mut [u8; 80], bit: u8) {
    reg.copy_within(1..80, 0);
    reg[79] = bit;
}

pub fn decode_ltc_stream(samples: &[f32], rate: f64, fps: f64, force_drop: bool) -> Option<Smpte> {
    if samples.len() < 32 || rate < 1000.0 || fps < 1.0 {
        return None;
    }
    let edges = transitions(samples);
    if edges.len() < 40 {
        return None;
    }
    let mut half = rate / (fps * 160.0);
    if half < 1.0 {
        return None;
    }
    let mut reg = [0u8; 80];
    let mut clean = 0usize;
    let mut pending = false;
    let mut last = edges[0];
    let mut found: Option<Smpte> = None;
    for &edge in &edges[1..] {
        let period = (edge - last) as f64;
        if period < half * 0.45 {
            continue;
        }
        last = edge;
        let short = period < half * 1.5;
        let long = !short && period < half * 3.0;
        if short {
            half = half * 0.92 + period * 0.08;
            if pending {
                push_bit(&mut reg, 1);
                clean += 1;
                pending = false;
                if let Some(smpte) =
                    locked_frame(&reg, clean, edge, samples.len(), half, fps, force_drop)
                {
                    found = Some(smpte);
                }
            } else {
                pending = true;
            }
        } else if long {
            half = half * 0.92 + period * 0.04;
            if pending {
                pending = false;
                clean = 0;
                reg = [0u8; 80];
            }
            push_bit(&mut reg, 0);
            clean += 1;
            if let Some(smpte) =
                locked_frame(&reg, clean, edge, samples.len(), half, fps, force_drop)
            {
                found = Some(smpte);
            }
        } else {
            pending = false;
            clean = 0;
            reg = [0u8; 80];
        }
    }
    found
}

pub(crate) fn locked_frame(
    reg: &[u8; 80],
    clean: usize,
    edge: usize,
    len: usize,
    half: f64,
    fps: f64,
    force_drop: bool,
) -> Option<Smpte> {
    if clean < 80 || reg[64..80] != SYNC {
        return None;
    }
    let mut smpte = decode_ltc_frame(reg, fps)?;
    let drop = force_drop || reg[10] == 1;
    smpte.total_frames = labels_to_total(smpte.hour, smpte.min, smpte.sec, smpte.frame, fps, drop);
    let frame_samples = (half * 160.0).max(1.0);
    let into = (len as f64 - (edge as f64 - frame_samples)) / frame_samples;
    smpte.frame_frac = (into - 1.0).clamp(0.0, 0.999);
    Some(smpte)
}
