

use crate::app::OtdApp;

pub(crate) const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "tga", "tif", "tiff", "ico", "ppm", "pgm", "pnm",
];
pub(crate) fn looks_like_an_image(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.iter().any(|k| e.eq_ignore_ascii_case(k)))
        .unwrap_or(false)
}
pub fn would_claim(app: &OtdApp, paths: &[std::path::PathBuf], at: Option<egui::Pos2>) -> bool {
    if !app.assistant.bar || app.assistant.collapsed || app.perform_preview_only() {
        return false;
    }
    let Some(at) = at else { return false };
    // A drop on the bar that is not an image is not ours; the media router
    // should have it rather than us silently eating it.
    app.assistant.bar_rect.contains(at) && paths.iter().any(|p| looks_like_an_image(p))
}
pub fn claim_drop(app: &mut OtdApp, paths: &[std::path::PathBuf], at: Option<egui::Pos2>) -> bool {
    if !would_claim(app, paths, at) {
        return false;
    }
    let Some(image) = paths.iter().find(|p| looks_like_an_image(p)).cloned() else {
        return false;
    };
    app.assistant.attach(&image);
    true
}
