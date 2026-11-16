//! TEA-ECB decryption for TouchDesigner project containers (WebToe / TD corpus).

pub const TEA_KEY: [u32; 4] = [0x9241fa3a, 0x59f32012, 0xc612aa09, 0xa98123d4];
const DELTA: u32 = 0x9e3779b9;

/// In-place decrypt over whole 8-byte blocks; trailing bytes (< 8) are left as-is.
pub fn decrypt(mut data: Vec<u8>, key: &[u32; 4]) -> Vec<u8> {
    let whole = data.len() - (data.len() % 8);
    let k0 = key[0];
    let k1 = key[1];
    let k2 = key[2];
    let k3 = key[3];
    for off in (0..whole).step_by(8) {
        let v0 = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        let v1 = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap());
        let (v0, v1) = decrypt_block(v0, v1, k0, k1, k2, k3);
        data[off..off + 4].copy_from_slice(&v0.to_le_bytes());
        data[off + 4..off + 8].copy_from_slice(&v1.to_le_bytes());
    }
    data
}

fn decrypt_block(mut v0: u32, mut v1: u32, k0: u32, k1: u32, k2: u32, k3: u32) -> (u32, u32) {
    let mut sum = DELTA.wrapping_mul(32);
    for _ in 0..32 {
        v1 = v1.wrapping_sub(
            ((v0 << 4).wrapping_add(k2)) ^ (v0.wrapping_add(sum)) ^ ((v0 >> 5).wrapping_add(k3)),
        );
        v0 = v0.wrapping_sub(
            ((v1 << 4).wrapping_add(k0)) ^ (v1.wrapping_add(sum)) ^ ((v1 >> 5).wrapping_add(k1)),
        );
        sum = sum.wrapping_sub(DELTA);
    }
    (v0, v1)
}

#[cfg(test)]
#[path = "tests/tea__tests.rs"]
mod tests;
