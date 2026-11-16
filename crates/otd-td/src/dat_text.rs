//! TouchDesigner `.text` sidecar bodies (binary header + script).

/// Decode a TD DAT `.text` sidecar into the script/source string.
pub fn decode_td_dat_text(bytes: &[u8]) -> String {
    if bytes.len() >= 3 && bytes[0] == b'2' && bytes[1] == b'\n' {
        // TD text DAT v2: `2\n*` + 24-byte header, optional length byte, then source.
        const V2_SOURCE: usize = 27;
        let body = if bytes.len() > V2_SOURCE {
            &bytes[V2_SOURCE..]
        } else {
            &[]
        };
        String::from_utf8_lossy(body)
            .trim_end_matches('\0')
            .to_string()
    } else if let Ok(s) = std::str::from_utf8(bytes) {
        s.trim_end_matches('\0').to_string()
    } else {
        String::from_utf8_lossy(bytes)
            .trim_end_matches('\0')
            .to_string()
    }
}

#[cfg(test)]
#[path = "tests/dat_text__tests.rs"]
mod tests;
