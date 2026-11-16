use otd_core::indexmap::IndexMap;
use otd_core::Param;

pub(crate) fn params_window_comp() -> IndexMap<String, Param> {
    params! {
        "winop" => Param::str("").with_label("Operator").as_path_ref().with_page("Window"),
        "title" => Param::str("Window").with_label("Title").with_page("Window"),
        "justifyoffsetto" => Param::menu("primarydisplay", &["primarydisplay", "specifydisplay", "alldisplays"])
            .with_label("Justify and Offset To")
            .with_page("Window"),
        "display" => Param::int(0).with_label("Display").with_range(0.0, 16.0).with_page("Window"),
        "justifyh" => Param::menu("center", &["left", "center", "right", "mouse"])
            .with_label("Justify Horizontal")
            .with_page("Window"),
        "justifyv" => Param::menu("center", &["top", "center", "bottom", "mouse"])
            .with_label("Justify Vertical")
            .with_page("Window"),
        "winoffsetx" => Param::int(0).with_label("Offset X").with_range(-8192.0, 8192.0).with_page("Window"),
        "winoffsety" => Param::int(0).with_label("Offset Y").with_range(-8192.0, 8192.0).with_page("Window"),
        "openingsize" => Param::menu("custom", &["automatic", "fill", "custom", "exclusive"])
            .with_label("Opening Size")
            .with_page("Window"),
        "winw" => Param::int(1280).with_label("Width").with_range(64.0, 16384.0).with_page("Window"),
        "winh" => Param::int(720).with_label("Height").with_range(64.0, 16384.0).with_page("Window"),
        "borders" => Param::bool(true).with_label("Borders").with_page("Window"),
        "alwaysontop" => Param::bool(false).with_label("Always on Top").with_page("Window"),
        "stretch" => Param::menu("fit", &["fit", "fill", "stretch", "native"])
            .with_label("Stretch")
            .with_page("Window"),
        "cursorvisible" => Param::bool(true).with_label("Cursor Visible").with_page("Window"),
        "allowminimize" => Param::bool(true).with_label("Allow Minimize").with_page("Window"),
        "vsyncmode" => Param::menu("enabled", &["disabled", "enabled", "halfmonitorrate"])
            .with_label("V-Sync Mode")
            .with_page("Window"),
        "open" => Param::bool(false).with_label("Open").with_page("Open/Close"),
        "performance" => Param::bool(false).with_label("Open as Perform Window").with_page("Open/Close"),
        "closeescape" => Param::bool(true).with_label("Close on Escape").with_page("Open/Close"),
        "blocksleep" => Param::bool(false).with_label("Prevent Display Sleep").with_page("Open/Close"),
    }
}