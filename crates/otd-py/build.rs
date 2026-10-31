fn main() {
    if std::env::var("OTD_BUNDLE_PYTHON").ok().as_deref() != Some("1") {
        return;
    }
    let subdir = std::env::var("OTD_PYTHON_RPATH_SUBDIR").unwrap_or_else(|_| "python".into());
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Resources/{subdir}/lib"
    );
}
