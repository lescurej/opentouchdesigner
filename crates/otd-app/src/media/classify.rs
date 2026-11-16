use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DropKind {
    Image,
    Movie,
    Audio,
    Project,
    Component,
    Isf,
    Shader(&'static str),
    Table(&'static str),
    Text,
    Font,
    TouchDesigner,
    Unknown,
}

impl DropKind {
    pub fn describe(self) -> &'static str {
        match self {
            DropKind::Image => "Movie File In — a still",
            DropKind::Movie => "Movie File In — plays on the timeline",
            DropKind::Audio => "Audio File In — a CHOP you can drive parameters with",
            DropKind::Project => "open this project",
            DropKind::Component => "import this component",
            DropKind::Isf => "GLSL TOP — ISF inputs become parameters",
            DropKind::Shader(_) => "GLSL TOP — the source, compiled live",
            DropKind::Table(_) => "Table DAT",
            DropKind::Text => "Text DAT",
            DropKind::Font => "Text TOP, in this font",
            DropKind::TouchDesigner => "import this TouchDesigner network",
            DropKind::Unknown => "not a file this can open",
        }
    }

    pub fn is_media(self) -> bool {
        matches!(self, DropKind::Image | DropKind::Movie | DropKind::Audio)
    }
}

pub const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "tga", "tif", "tiff"];
pub const MOVIE_EXT: &[&str] = &["mp4", "mov", "m4v", "mkv", "webm", "avi", "gif"];
pub const AUDIO_EXT: &[&str] = &["wav", "mp3", "aiff", "aif", "flac", "ogg", "m4a", "aac"];

pub fn classify(path: &Path) -> DropKind {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        e if IMAGE_EXT.contains(&e) => DropKind::Image,
        e if MOVIE_EXT.contains(&e) => DropKind::Movie,
        e if AUDIO_EXT.contains(&e) => DropKind::Audio,
        "otd" => DropKind::Project,
        "otdc" => DropKind::Component,
        "fs" | "isf" => DropKind::Isf,
        "frag" | "glsl" => DropKind::Shader("glsl"),
        "wgsl" => DropKind::Shader("wgsl"),
        "csv" => DropKind::Table("comma"),
        "tsv" => DropKind::Table("tab"),
        "txt" | "md" | "json" | "py" | "log" => DropKind::Text,
        "ttf" | "otf" => DropKind::Font,
        "tox" | "toe" => DropKind::TouchDesigner,
        _ => DropKind::Unknown,
    }
}