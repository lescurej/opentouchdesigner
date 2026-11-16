use crate::Point;

use super::types::Attrib;

/// Decrypt one `P` block. `seed` is `0x32` for the first point and, after that,
/// the last plaintext byte returned here.
pub(crate) fn decrypt_p(block: &[u8], seed: u8) -> Result<([f32; 4], u8), String> {
    if block.len() != 16 {
        return Err("btog: P block is not 16 bytes".into());
    }
    let (w, prev) = decrypt_f32(&block[12..16], seed);
    let (z, prev) = decrypt_f32(&block[8..12], prev);
    let (y, prev) = decrypt_f32(&block[4..8], prev);
    let (x, prev) = decrypt_f32(&block[0..4], prev);
    Ok(([x, y, z, w], prev))
}

pub(crate) fn decrypt_f32(bytes: &[u8], mut prev: u8) -> (f32, u8) {
    let mut out = [0u8; 4];
    for (slot, enc) in out.iter_mut().zip(bytes) {
        let plain = enc ^ prev.wrapping_mul(17).wrapping_add(3);
        *slot = plain;
        prev = plain;
    }
    (f32::from_be_bytes(out), prev)
}

pub(crate) fn apply_attr(point: &mut Point, attr: &Attrib, raw: &[u8]) {
    if !matches!(attr.type_code, 0 | 5) {
        return;
    }
    let f = |i: usize| -> Option<f32> {
        let start = i.checked_mul(4)?;
        let bytes: [u8; 4] = raw.get(start..start + 4)?.try_into().ok()?;
        Some(f32::from_be_bytes(bytes))
    };
    match attr.name.as_str() {
        "N" => {
            if let (Some(x), Some(y), Some(z)) = (f(0), f(1), f(2)) {
                point.normal = [x, y, z];
            }
        }
        "uv" => {
            if let (Some(u), Some(v)) = (f(0), f(1)) {
                point.uv = [u, v];
            }
        }
        "Cd" => {
            if let (Some(r), Some(g), Some(b)) = (f(0), f(1), f(2)) {
                let a = f(3).unwrap_or(point.color[3]);
                point.color = [r, g, b, a];
            }
        }
        "Alpha" => {
            if let Some(a) = f(0) {
                point.color[3] = a;
            }
        }
        _ => {}
    }
}
