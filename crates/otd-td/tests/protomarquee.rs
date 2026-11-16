//! Validates native decode against `toeexpand` when the fixture is present.

use std::path::Path;

use otd_td::{assess_expansion, container_offset, decode_container, load_dir};

#[test]
fn protomarquee_native_decode_if_present() {
    let tox = Path::new("/tmp/protomarquee.tox");
    if !tox.is_file() {
        return;
    }
    let bytes = std::fs::read(tox).unwrap();
    assert!(container_offset(&bytes).is_some());
    let (native, _) = decode_container(&bytes).expect("native decode");
    assess_expansion(&native).unwrap();
    let dir = otd_td::expansion_dir(tox);
    if !dir.is_dir() {
        return;
    }
    let expanded = load_dir(&dir).unwrap();
    assert_eq!(native.len(), expanded.len(), "file count vs toeexpand");
}

