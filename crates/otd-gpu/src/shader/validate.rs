pub fn validate_wgsl(source: &str) -> Result<(), String> {
    use wgpu::naga;
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| trim_location(&e.emit_to_string(source)))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .map_err(|e| trim_location(&e.emit_to_string(source)))?;
    Ok(())
}

pub fn validate_glsl(source: &str) -> Result<(), String> {
    use wgpu::naga;
    let mut frontend = naga::front::glsl::Frontend::default();
    let options = naga::front::glsl::Options::from(naga::ShaderStage::Fragment);
    let module = frontend
        .parse(&options, source)
        .map_err(|e| trim_location(&e.emit_to_string(source)))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .map_err(|e| trim_location(&e.emit_to_string(source)))?;
    Ok(())
}

/// naga's diagnostics are several lines of source excerpt. The parameter
/// panel has one line, so keep the first two and drop the ASCII art.
fn trim_location(message: &str) -> String {
    let useful: Vec<&str> = message
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('|') && !l.chars().all(|c| "-^ ".contains(c)))
        .take(3)
        .collect();
    if useful.is_empty() {
        message.trim().to_string()
    } else {
        useful.join(" — ")
    }
}
