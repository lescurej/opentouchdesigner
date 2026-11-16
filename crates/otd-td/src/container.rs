//! Native `.toe` / `.tox` container decoder (format aligned with WebToe's `toeBinary.ts`).

use std::collections::BTreeMap;
use std::io::Read;

use flate2::bufread::ZlibDecoder;
use thiserror::Error;

use crate::tea::{TEA_KEY, decrypt};

const TAG_FILE: u8 = 0x34;
const TAG_PUSH: u8 = 0x36;
const TAG_POP: u8 = 0x35;
const TRAILER: &str = "TRAILER!!!";

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("not a TouchDesigner container")]
    NotContainer,
    #[error("segment {which}: {detail}")]
    Segment { which: usize, detail: String },
    #[error("zlib: {0}")]
    Zlib(String),
    #[error("decoded no files")]
    Empty,
}

#[derive(Debug, Clone)]
pub struct DecodeMeta {
    pub segments: usize,
    pub kinds: Vec<String>,
    pub recovered: usize,
}

pub fn is_segment_at(bytes: &[u8], o: usize) -> bool {
    bytes.len() > o + 1 && bytes[o] == b'1' && bytes[o + 1].is_ascii_digit()
}

/// Some `.tox` files prefix the segment stream with a 4-byte big-endian header.
pub fn container_offset(bytes: &[u8]) -> Option<usize> {
    if is_segment_at(bytes, 0) {
        return Some(0);
    }
    if bytes.len() > 10 && is_segment_at(bytes, 4) {
        return Some(4);
    }
    (1..bytes.len().saturating_sub(2)).find(|&o| is_segment_at(bytes, o))
}

pub fn is_toe_container(bytes: &[u8]) -> bool {
    container_offset(bytes).is_some()
}

fn be32(bytes: &[u8], o: usize) -> u32 {
    u32::from_be_bytes(bytes[o..o + 4].try_into().unwrap())
}

fn inflate_zlib(cipher_padded: &[u8]) -> Result<Vec<u8>, ContainerError> {
    let plain = decrypt(cipher_padded.to_vec(), &TEA_KEY);
    let mut decoder = ZlibDecoder::new(plain.as_slice());
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| ContainerError::Zlib(e.to_string()))?;
    Ok(out)
}

struct ParsedSegment {
    entries: Vec<(String, Vec<u8>)>,
    stack: Vec<String>,
    closed: bool,
}

fn parse_records(blob: &[u8], stack_in: &[String]) -> Option<ParsedSegment> {
    let mut stack: Vec<String> = stack_in.to_vec();
    let mut entries = Vec::new();
    let mut i = 0;
    let mut closed = false;
    while i < blob.len() {
        if closed {
            return None;
        }
        let tag = blob[i];
        match tag {
            TAG_FILE => {
                if i + 6 > blob.len() {
                    return None;
                }
                let name_len = blob[i + 1] as usize;
                let len = be32(blob, i + 2) as usize;
                i += 6;
                if name_len < 1 || i + name_len + len > blob.len() || blob[i + name_len - 1] != 0 {
                    return None;
                }
                let name = std::str::from_utf8(&blob[i..i + name_len - 1]).ok()?;
                i += name_len;
                let content = blob[i..i + len].to_vec();
                i += len;
                if name == TRAILER {
                    closed = true;
                } else {
                    let path = if stack.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}/{}", stack.join("/"), name)
                    };
                    entries.push((path, content));
                }
            }
            TAG_PUSH => {
                if i + 3 > blob.len() {
                    return None;
                }
                let name_len = blob[i + 2] as usize;
                i += 3;
                if name_len < 1 || i + name_len > blob.len() || blob[i + name_len - 1] != 0 {
                    return None;
                }
                let name = std::str::from_utf8(&blob[i..i + name_len - 1]).ok()?;
                stack.push(name.to_string());
                i += name_len;
            }
            TAG_POP => {
                if stack.is_empty() {
                    return None;
                }
                stack.pop();
                i += 1;
            }
            _ => return None,
        }
    }
    Some(ParsedSegment {
        entries,
        stack,
        closed,
    })
}

fn unique_path(files: &BTreeMap<String, Vec<u8>>, path: String) -> String {
    if !files.contains_key(&path) {
        return path;
    }
    let mut n = 2;
    loop {
        let candidate = format!("{path}.{n}");
        if !files.contains_key(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

struct SegmentBlob {
    blob: Vec<u8>,
    next: usize,
}

fn framed_segment(bytes: &[u8], o: usize) -> Option<SegmentBlob> {
    let len = bytes.len();
    if o + 2 > len {
        return None;
    }
    let kind = bytes[o + 1].wrapping_sub(b'0');
    let mut chunks: Vec<(usize, u32, u32)> = Vec::new();
    let next = if kind == 0 {
        if o + 10 > len {
            return None;
        }
        chunks.push((o + 10, be32(bytes, o + 2), be32(bytes, o + 6)));
        o + 10 + chunks[0].1 as usize
    } else if kind == 2 {
        if o + 18 > len {
            return None;
        }
        let count = be32(bytes, o + 2);
        if !(1..=65536).contains(&count) {
            return None;
        }
        let mut at = o + 18;
        let mut clen = be32(bytes, o + 10);
        let mut ulen = be32(bytes, o + 14);
        for i in 0..count {
            chunks.push((at, clen, ulen));
            at += clen as usize;
            if i + 1 < count {
                if at + 8 > len {
                    return None;
                }
                clen = be32(bytes, at);
                ulen = be32(bytes, at + 4);
                at += 8;
            }
        }
        at
    } else {
        return None;
    };
    if next > len || (next < len && !is_segment_at(bytes, next)) {
        return None;
    }
    let mut outs = Vec::new();
    for (at, clen, ulen) in chunks {
        if clen < 2 || at + clen as usize > len {
            return None;
        }
        let blob = inflate_zlib(&bytes[at..at + clen as usize]).ok()?;
        if blob.len() as u32 != ulen {
            return None;
        }
        outs.push(blob);
    }
    let blob = outs.concat();
    Some(SegmentBlob { blob, next })
}

/// Decode a `.toe` / `.tox` file from bytes into the same paths `toeexpand` writes.
pub fn decode_container(
    bytes: &[u8],
) -> Result<(BTreeMap<String, Vec<u8>>, DecodeMeta), ContainerError> {
    let start = container_offset(bytes).ok_or(ContainerError::NotContainer)?;
    let mut files = BTreeMap::new();
    let mut kinds = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut segments = 0usize;
    let recovered = 0usize;
    let mut closed = false;
    let mut o = start;
    while o < bytes.len() && !closed {
        if !is_segment_at(bytes, o) {
            return Err(ContainerError::Segment {
                which: segments + 1,
                detail: format!("no segment magic at byte {o}"),
            });
        }
        let kind = format!("{}{}", bytes[o] as char, bytes[o + 1] as char);
        kinds.push(kind);
        let seg = framed_segment(bytes, o).ok_or_else(|| ContainerError::Segment {
            which: segments + 1,
            detail: "length fields did not decode (recovery not implemented yet)".into(),
        })?;
        segments += 1;
        let parsed = parse_records(&seg.blob, &stack).ok_or_else(|| ContainerError::Segment {
            which: segments,
            detail: "records do not parse".into(),
        })?;
        stack = parsed.stack;
        closed = parsed.closed;
        for (path, content) in parsed.entries {
            let path = unique_path(&files, path);
            files.insert(path, content);
        }
        o = seg.next;
    }
    if files.is_empty() {
        return Err(ContainerError::Empty);
    }
    kinds.sort();
    kinds.dedup();
    Ok((
        files,
        DecodeMeta {
            segments,
            kinds,
            recovered,
        },
    ))
}

/// Sanity check — same idea as WebToe's `assessToeExpansion`.
pub fn assess_expansion(files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    if !files.contains_key(".build") {
        return Err("no .build record".into());
    }
    if !files.keys().any(|p| p.ends_with(".n")) {
        return Err("no node (.n) records".into());
    }
    Ok(())
}
