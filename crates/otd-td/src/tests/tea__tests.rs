use super::*;

    #[test]
    fn round_trip_encrypt_decrypt_block() {
        let key = TEA_KEY;
        let plain = vec![0u8; 16];
        let mut enc = plain.clone();
        // reference encrypt (mirrors WebToe tests)
        let whole = enc.len();
        for off in (0..whole).step_by(8) {
            let mut v0 = u32::from_le_bytes(enc[off..off + 4].try_into().unwrap());
            let mut v1 = u32::from_le_bytes(enc[off + 4..off + 8].try_into().unwrap());
            let (k0, k1, k2, k3) = (key[0], key[1], key[2], key[3]);
            let mut sum = 0u32;
            for _ in 0..32 {
                sum = sum.wrapping_add(DELTA);
                v0 = v0.wrapping_add(
                    ((v1 << 4).wrapping_add(k0))
                        ^ (v1.wrapping_add(sum))
                        ^ ((v1 >> 5).wrapping_add(k1)),
                );
                v1 = v1.wrapping_add(
                    ((v0 << 4).wrapping_add(k2))
                        ^ (v0.wrapping_add(sum))
                        ^ ((v0 >> 5).wrapping_add(k3)),
                );
            }
            enc[off..off + 4].copy_from_slice(&v0.to_le_bytes());
            enc[off + 4..off + 8].copy_from_slice(&v1.to_le_bytes());
        }
        let dec = decrypt(enc, &key);
        assert_eq!(dec, plain);
    }
