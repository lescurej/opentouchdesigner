pub fn nothing_to_open(is_camera: bool, source: &str) -> bool {
    !is_camera && source.trim().is_empty()
}