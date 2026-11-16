pub const MAGIC: &[u8; 8] = b"BtogV\0\0\0";

pub(crate) const P_CHAIN_SEED: u8 = 0x32;
pub(crate) const HEADER_LEN: usize = 41;

pub(crate) fn header_version(data: &[u8]) -> Option<u32> {
    if data.len() < 16 || data[..4] != *b"Btog" || data[4] != b'V' {
        return None;
    }
    let bytes: [u8; 4] = data.get(5..9)?.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}
