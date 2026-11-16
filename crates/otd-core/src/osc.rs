pub fn encode_message(address: &str, args: &[OscArg]) -> Result<Vec<u8>, String> {
    let address = address.trim();
    if address.is_empty() || !address.starts_with('/') {
        return Err("OSC address must start with '/'".into());
    }
    let mut types = String::from(",");
    let mut data = Vec::new();
    for arg in args {
        match arg {
            OscArg::Int(v) => {
                types.push('i');
                data.extend_from_slice(&v.to_be_bytes());
            }
            OscArg::Float(v) => {
                types.push('f');
                data.extend_from_slice(&v.to_be_bytes());
            }
            OscArg::Str(s) => {
                types.push('s');
                append_padded(&mut data, s.as_bytes());
            }
            OscArg::Blob(b) => {
                types.push('b');
                data.extend_from_slice(&(b.len() as i32).to_be_bytes());
                data.extend_from_slice(b);
                pad4(&mut data);
            }
        }
    }
    let mut out = Vec::new();
    append_padded(&mut out, address.as_bytes());
    append_padded(&mut out, types.as_bytes());
    out.extend_from_slice(&data);
    Ok(out)
}

#[derive(Clone, Debug)]
pub enum OscArg {
    Int(i32),
    Float(f32),
    Str(String),
    Blob(Vec<u8>),
}

fn pad4(buf: &mut Vec<u8>) {
    let pad = (4 - (buf.len() % 4)) % 4;
    buf.extend(std::iter::repeat_n(0u8, pad));
}

fn append_padded(buf: &mut Vec<u8>, s: &[u8]) {
    buf.extend_from_slice(s);
    buf.push(0);
    pad4(buf);
}

#[cfg(test)]
#[path = "tests/osc__tests.rs"]
mod tests;
