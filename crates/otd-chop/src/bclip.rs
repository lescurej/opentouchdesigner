use std::fs;
use std::path::Path;

use crate::chopfile;
use crate::data::{Channel, ChopData};

const META_TAG: u16 = 0x000f;
const CHOP_TAG: u16 = 0x1000;
const NAME_TAG: u16 = 0x0010;

pub fn load(path: &Path) -> Result<ChopData, String> {
    let data = fs::read(path).map_err(|e| format!("{path:?}: {e}"))?;
    parse(&data).map_err(|e| format!("{path:?}: {e}"))
}

pub fn parse(data: &[u8]) -> Result<ChopData, String> {
    if data.len() < 8 {
        return Err("file too short".into());
    }
    let magic = &data[0..4];
    if magic != b"bcli" && magic != b"bclp" {
        return Err(format!("bad magic `{}`", String::from_utf8_lossy(magic)));
    }

    let mut rate = crate::data::CONTROL_RATE;
    let mut clip_payload: Option<&[u8]> = None;

    let mut o = 4usize;
    while o + 8 <= data.len() {
        let rec_len = read_u32_be(data, o)? as usize;
        if rec_len < 8 || o + rec_len > data.len() {
            break;
        }
        let tag = read_u16_be(data, o + 4)?;
        let id = read_u16_be(data, o + 6)?;
        let pay = &data[o + 8..o + rec_len];
        if tag == META_TAG {
            match id {
                1 if pay.len() >= 4 => rate = read_f32_be(pay, 0)? as f64,
                5 => clip_payload = Some(pay),
                _ => {}
            }
        }
        o += rec_len;
    }

    let body = clip_payload.ok_or_else(|| "clip payload missing".to_string())?;
    let channels = parse_clip_body(body)?;
    if channels.is_empty() {
        return Err("no channels".into());
    }
    Ok(ChopData::new(channels, rate.max(1.0), false))
}

fn parse_clip_body(body: &[u8]) -> Result<Vec<Channel>, String> {
    if body.len() < 8 {
        return Err("clip body too short".into());
    }
    let nch = read_u32_be(body, 0)? as usize;
    if nch == 0 || nch > 10_000 {
        return Err(format!("bad channel count {nch}"));
    }
    let mut o = 8usize;
    let mut out = Vec::with_capacity(nch);
    for i in 0..nch {
        let (name, samples, next) = parse_channel(body, o)?;
        o = next;
        out.push(Channel::new(name, samples));
        if i + 1 < nch {
            o = skip_channel_separator(body, o)?;
        }
    }
    Ok(out)
}

fn parse_channel(body: &[u8], mut o: usize) -> Result<(String, Vec<f32>, usize), String> {
    while o + 6 <= body.len() {
        let tag = read_u16_be(body, o)?;
        let id = read_u16_be(body, o + 2)?;
        if tag == NAME_TAG && id == 1 {
            break;
        }
        o += 1;
    }
    if o + 6 > body.len() {
        return Err("channel name header not found".into());
    }
    let channel_start = o;
    let nlen = read_u16_be(body, o + 4)? as usize;
    o += 6;
    if o + nlen > body.len() {
        return Err("truncated channel name".into());
    }
    let name = std::str::from_utf8(&body[o..o + nlen])
        .map_err(|_| "channel name is not utf-8".to_string())?
        .to_string();
    o = align_up(channel_start + 6 + nlen, 8);
    if o > body.len() {
        return Err("bad channel name block".into());
    }
    o = advance_to_sample_block(body, o)?;
    let (samples, next) = parse_sample_block(body, o)?;
    Ok((name, samples, next))
}

fn align_up(v: usize, a: usize) -> usize {
    debug_assert!(a.is_power_of_two());
    (v + a - 1) & !(a - 1)
}

fn advance_to_sample_block(body: &[u8], o: usize) -> Result<usize, String> {
    let end = o.saturating_add(64).min(body.len());
    for pos in o..end {
        if sample_header_valid(body, pos)? {
            return Ok(pos);
        }
    }
    Err("missing sample block".into())
}

fn sample_header_valid(body: &[u8], o: usize) -> Result<bool, String> {
    if o + 8 > body.len() {
        return Ok(false);
    }
    let blen = read_u32_be(body, o)? as usize;
    if blen < 8 || o + blen > body.len() {
        return Ok(false);
    }
    if read_u16_le(body, o + 4)? != CHOP_TAG {
        return Ok(false);
    }
    if read_u16_be(body, o + 6)? != 2 {
        return Ok(false);
    }
    Ok((blen - 8) % 4 == 0 && (blen - 8) / 4 > 0)
}

fn parse_sample_block(body: &[u8], o: usize) -> Result<(Vec<f32>, usize), String> {
    if o + 8 > body.len() {
        return Err("truncated sample header".into());
    }
    let blen = read_u32_be(body, o)? as usize;
    if blen < 8 || o + blen > body.len() {
        return Err(format!("bad sample block length {blen}"));
    }
    let tag = read_u16_le(body, o + 4)?;
    let id = read_u16_be(body, o + 6)?;
    if tag != CHOP_TAG || id != 2 {
        return Err(format!("expected sample block, got {tag:#x}/{id}"));
    }
    let pay_len = blen.saturating_sub(8);
    if pay_len % 4 != 0 {
        return Err("sample payload size is not a multiple of 4".into());
    }
    let nf = pay_len / 4;
    if nf == 0 {
        return Err("sample block has no floats".into());
    }
    let mut samples = Vec::with_capacity(nf);
    let mut p = o + 8;
    for _ in 0..nf {
        samples.push(read_f32_be(body, p)?);
        p += 4;
    }
    Ok((samples, o + blen))
}

fn skip_channel_separator(body: &[u8], o: usize) -> Result<usize, String> {
    let end = o.saturating_add(16).min(body.len());
    for pos in o..end {
        if pos + 8 > body.len() {
            break;
        }
        let rec_len = read_u16_le(body, pos)? as usize;
        let tag = read_u16_be(body, pos + 2)?;
        let id = read_u16_le(body, pos + 4)?;
        if rec_len == 8 && tag == CHOP_TAG && id == 0 {
            return Ok(pos + rec_len);
        }
    }
    Err("bad channel separator".into())
}

pub fn load_any(path: &Path) -> Result<ChopData, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("bclip") || ext.eq_ignore_ascii_case("bclp") {
        return load(path);
    }
    let data = fs::read(path).map_err(|e| format!("{path:?}: {e}"))?;
    if data.len() >= 4 && (&data[0..4] == b"bcli" || &data[0..4] == b"bclp") {
        return parse(&data).map_err(|e| format!("{path:?}: {e}"));
    }
    chopfile::load(path)
}

fn read_u32_be(data: &[u8], o: usize) -> Result<u32, String> {
    data.get(o..o + 4)
        .ok_or_else(|| "unexpected end".to_string())
        .and_then(|b| {
            b.try_into()
                .map(u32::from_be_bytes)
                .map_err(|_| "read u32".into())
        })
}

fn read_u16_be(data: &[u8], o: usize) -> Result<u16, String> {
    data.get(o..o + 2)
        .ok_or_else(|| "unexpected end".to_string())
        .and_then(|b| {
            b.try_into()
                .map(u16::from_be_bytes)
                .map_err(|_| "read u16".into())
        })
}

fn read_u16_le(data: &[u8], o: usize) -> Result<u16, String> {
    data.get(o..o + 2)
        .ok_or_else(|| "unexpected end".to_string())
        .and_then(|b| {
            b.try_into()
                .map(u16::from_le_bytes)
                .map_err(|_| "read u16".into())
        })
}

fn read_f32_be(data: &[u8], o: usize) -> Result<f32, String> {
    Ok(f32::from_be_bytes(
        data.get(o..o + 4)
            .ok_or_else(|| "unexpected end".to_string())?
            .try_into()
            .map_err(|_| "read f32".to_string())?,
    ))
}

#[cfg(test)]
#[path = "tests/bclip__tests.rs"]
mod tests;
