use std::path::PathBuf;

fn main() {
    let media = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/media");
    println!("cargo:rustc-env=OTD_MEDIA_DIR={}", media.display());
    println!("cargo:rerun-if-changed=../../examples/media/apple.png");
}
