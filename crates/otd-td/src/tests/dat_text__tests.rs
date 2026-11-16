use super::*;
use std::fs;

    #[test]
    fn protomarquee_text1_has_no_binary_prefix() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protomarquee.tox.dir/protomarquee/geo1/scale/text1.text"
        );
        let Ok(bytes) = fs::read(path) else {
            return;
        };
        let text = decode_td_dat_text(&bytes);
        assert!(
            text.starts_with("for c in ops"),
            "decoded text should start with script, got {:?}",
            &text[..text.len().min(40)]
        );
        assert!(!text.contains('\0'));
    }
