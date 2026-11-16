#[derive(Clone, Copy, Debug)]
pub struct Smpte {
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub frame: u32,
    pub frame_frac: f64,
    pub total_frames: i64,
}

pub(crate) const SYNC: [u8; 16] = [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1];

pub fn smpte_from_seconds(t: f64, fps: f64, drop_frame: bool) -> Smpte {
    let t = t.max(0.0);
    let fps = fps.max(1.0);
    let drop = drop_frame && ((fps - 29.97).abs() < 0.05 || (fps - 30.0).abs() < 0.05);
    if drop {
        let fps_df = 30000.0 / 1001.0;
        let exact = t * fps_df;
        let mut s = smpte_from_drop_frames(exact.floor() as i64);
        s.frame_frac = exact.fract();
        s
    } else {
        let exact = t * fps;
        let mut s = smpte_non_drop_frames(exact.floor() as i64, fps);
        s.frame_frac = exact.fract();
        s
    }
}

pub(crate) fn smpte_non_drop_frames(total: i64, fps: f64) -> Smpte {
    let fps_i = fps.round().max(1.0) as i64;
    let total = total.max(0);
    let frame = (total % fps_i) as u32;
    let total_sec = total / fps_i;
    let sec = (total_sec % 60) as u32;
    let total_min = total_sec / 60;
    let min = (total_min % 60) as u32;
    let hour = ((total_min / 60) % 24) as u32;
    Smpte {
        hour,
        min,
        sec,
        frame,
        frame_frac: 0.0,
        total_frames: total,
    }
}

pub(crate) fn smpte_from_drop_frames(frames: i64) -> Smpte {
    let frames = frames.max(0);
    let ten = frames / 17982;
    let rem = frames % 17982;
    let extra = if rem < 2 { 0 } else { 2 * ((rem - 2) / 1798) };
    let adjusted = frames + 18 * ten + extra;
    let frame = (adjusted % 30) as u32;
    let total_sec = adjusted / 30;
    let sec = (total_sec % 60) as u32;
    let total_min = total_sec / 60;
    let min = (total_min % 60) as u32;
    let hour = ((total_min / 60) % 24) as u32;
    Smpte {
        hour,
        min,
        sec,
        frame,
        frame_frac: 0.0,
        total_frames: frames,
    }
}

pub(crate) fn labels_to_total(hour: u32, min: u32, sec: u32, frame: u32, fps: f64, drop: bool) -> i64 {
    if drop {
        let nominal = ((hour as i64 * 60 + min as i64) * 60 + sec as i64) * 30 + frame as i64;
        let minutes = hour as i64 * 60 + min as i64;
        nominal - 2 * (minutes - minutes / 10)
    } else {
        let fps_i = fps.round().max(1.0) as i64;
        ((hour as i64 * 60 + min as i64) * 60 + sec as i64) * fps_i + frame as i64
    }
}

fn write_bcd(bits: &mut [u8; 80], start: usize, value: u32, width: usize) {
    for i in 0..width {
        bits[start + i] = ((value >> i) & 1) as u8;
    }
}

pub(super) fn read_bcd(bits: &[u8; 80], start: usize, width: usize) -> u32 {
    let mut v = 0u32;
    for i in 0..width {
        v |= (bits[start + i] as u32) << i;
    }
    v
}

fn set_parity(bits: &mut [u8; 80], fps: f64) {
    let bit = if (fps - 25.0).abs() < 0.1 { 59 } else { 27 };
    bits[bit] = 0;
    let ones = bits.iter().filter(|&&b| b == 1).count();
    if ones % 2 == 1 {
        bits[bit] = 1;
    }
}

const USER_BIT: [usize; 8] = [4, 12, 20, 28, 36, 44, 52, 60];

#[cfg(test)]
pub fn encode_ltc_frame(smpte: &Smpte, drop_frame: bool, fps: f64) -> [u8; 80] {
    encode_ltc_frame_user(smpte, drop_frame, fps, &[0; 8])
}

pub fn encode_ltc_frame_user(
    smpte: &Smpte,
    drop_frame: bool,
    fps: f64,
    user: &[u8; 8],
) -> [u8; 80] {
    let mut b = [0u8; 80];
    let (frame, high) = if fps > 30.5 && smpte.frame >= 30 {
        (smpte.frame - 30, true)
    } else {
        (smpte.frame, false)
    };
    write_bcd(&mut b, 0, frame % 10, 4);
    write_bcd(&mut b, 8, frame / 10, 2);
    if drop_frame {
        b[10] = 1;
    }
    write_bcd(&mut b, 16, smpte.sec % 10, 4);
    write_bcd(&mut b, 24, smpte.sec / 10, 3);
    write_bcd(&mut b, 32, smpte.min % 10, 4);
    write_bcd(&mut b, 40, smpte.min / 10, 3);
    write_bcd(&mut b, 48, smpte.hour % 10, 4);
    write_bcd(&mut b, 56, smpte.hour / 10, 2);
    for (start, value) in USER_BIT.iter().zip(user) {
        write_bcd(&mut b, *start, (*value as u32) & 0xf, 4);
    }
    if high {
        b[4] = 1;
    }
    b[64..80].copy_from_slice(&SYNC);
    set_parity(&mut b, fps);
    b
}

pub fn smpte_from_labels(
    hour: u32,
    min: u32,
    sec: u32,
    frame: u32,
    frame_frac: f64,
    fps: f64,
    drop: bool,
) -> Smpte {
    let hour = hour % 24;
    let min = min % 60;
    let sec = sec % 60;
    let max_frame = if drop {
        30
    } else {
        fps.round().max(1.0) as u32
    };
    let frame = frame % max_frame.max(1);
    Smpte {
        hour,
        min,
        sec,
        frame,
        frame_frac: frame_frac.clamp(0.0, 0.999),
        total_frames: labels_to_total(hour, min, sec, frame, fps, drop),
    }
}

pub fn parse_timecode(text: &str, fps: f64, drop: bool) -> Option<Smpte> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let drop = drop || text.contains(';');
    let parts: Vec<&str> = text.split([':', ';']).collect();
    if parts.len() != 4 {
        return None;
    }
    let hour: u32 = parts[0].trim().parse().ok()?;
    let min: u32 = parts[1].trim().parse().ok()?;
    let sec: u32 = parts[2].trim().parse().ok()?;
    let frame: u32 = parts[3].trim().parse().ok()?;
    if hour >= 24 || min >= 60 || sec >= 60 {
        return None;
    }
    Some(smpte_from_labels(hour, min, sec, frame, 0.0, fps, drop))
}

pub fn decode_ltc_frame(bits: &[u8; 80], fps: f64) -> Option<Smpte> {
    if bits[64..80] != SYNC {
        return None;
    }
    let mut frame = read_bcd(bits, 0, 4) + read_bcd(bits, 8, 2) * 10;
    if fps > 30.5 && bits[4] == 1 {
        frame += 30;
    }
    let sec = read_bcd(bits, 16, 4) + read_bcd(bits, 24, 3) * 10;
    let min = read_bcd(bits, 32, 4) + read_bcd(bits, 40, 3) * 10;
    let hour = read_bcd(bits, 48, 4) + read_bcd(bits, 56, 2) * 10;
    let max_frame = if fps > 30.5 {
        60
    } else {
        fps.round().max(1.0) as u32
    };
    if frame >= max_frame || sec >= 60 || min >= 60 || hour >= 24 {
        return None;
    }
    Some(Smpte {
        hour,
        min,
        sec,
        frame,
        frame_frac: 0.0,
        total_frames: 0,
    })
}

