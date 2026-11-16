use otd_core::indexmap::IndexMap;
use otd_core::Param;

pub const SCREEN_GRAB: &str = "screengrabTOP";
pub const VIDEO_STREAM_IN: &str = "videostreaminTOP";
pub const VIDEO_STREAM_OUT: &str = "videostreamoutTOP";

pub fn is_syphon_top(type_name: &str) -> bool {
    matches!(type_name, "syphoninTOP" | "syphonoutTOP")
}

pub fn is_spout_top(type_name: &str) -> bool {
    matches!(type_name, "spoutinTOP" | "spoutoutTOP")
}

pub fn io_stub_status(type_name: &str) -> Option<&'static str> {
    match type_name {
        "syphoninTOP" | "syphonoutTOP" => {
            #[cfg(not(target_os = "macos"))]
            {
                Some("Syphon is macOS only.")
            }
            #[cfg(target_os = "macos")]
            Some("Syphon needs the Syphon framework — not linked in this build yet.")
        }
        "spoutinTOP" | "spoutoutTOP" => {
            #[cfg(not(target_os = "windows"))]
            {
                Some("Spout is Windows only.")
            }
            #[cfg(target_os = "windows")]
            Some("Spout needs the Spout SDK — not linked in this build yet.")
        }
        VIDEO_STREAM_OUT => {
            Some("Video Stream Out needs an RTMP/SRT publisher — not implemented yet.")
        }
        _ => None,
    }
}

pub(crate) fn params_screen_grab() -> IndexMap<String, Param> {
    params! {
        "screen" => Param::int(0).with_label("Screen Index").with_range(0.0, 8.0),
        "resw" => Param::int(1280).with_label("Requested W").with_range(1.0, 16384.0),
        "resh" => Param::int(720).with_label("Requested H").with_range(1.0, 16384.0),
        "fps" => Param::float(30.0).with_label("Requested Frame Rate").with_range(1.0, 120.0),
        "active" => Param::bool(true).with_label("Active"),
    }
}

pub(crate) fn params_videostream_in() -> IndexMap<String, Param> {
    params! {
        "url" => Param::str("").with_label("URL"),
        "active" => Param::bool(true).with_label("Active"),
    }
}

pub(crate) fn params_videostream_out() -> IndexMap<String, Param> {
    params! {
        "url" => Param::str("").with_label("URL"),
        "active" => Param::bool(false).with_label("Active"),
    }
}