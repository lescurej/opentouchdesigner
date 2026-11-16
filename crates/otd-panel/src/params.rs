use otd_core::Param;
use otd_core::indexmap::IndexMap;

pub fn append_layout_page(m: &mut IndexMap<String, Param>, defaults: (f64, f64, f64, f64)) {
    let (dx, dy, dw, dh) = defaults;
    for (key, label, default) in [
        ("x", "X", dx),
        ("y", "Y", dy),
        ("w", "Width", dw),
        ("h", "Height", dh),
    ] {
        m.insert(
            key.into(),
            Param::float(default)
                .with_label(label)
                .with_range(0.0, 1.0)
                .with_page("Layout"),
        );
    }
    m.insert(
        "fixedaspect".into(),
        Param::menu("off", &["off", "horizontal", "vertical"])
            .with_label("Fixed Aspect")
            .with_page("Layout"),
    );
    m.insert(
        "aspect".into(),
        Param::float(1.0)
            .with_label("Aspect Ratio")
            .with_range(0.01, 16.0)
            .with_page("Layout"),
    );
    m.insert(
        "layer".into(),
        Param::float(0.0)
            .with_label("Depth Layer")
            .with_page("Layout"),
    );
    m.insert(
        "hmode".into(),
        Param::menu("fixed", &["fixed", "fill", "anchors"])
            .with_label("Horizontal Mode")
            .with_page("Layout"),
    );
    m.insert(
        "leftanchor".into(),
        Param::float(0.0)
            .with_label("Left Anchor")
            .with_range(0.0, 1.0)
            .with_page("Layout"),
    );
    m.insert(
        "leftoffset".into(),
        Param::float(0.0)
            .with_label("Left Offset")
            .with_page("Layout"),
    );
    m.insert(
        "rightanchor".into(),
        Param::float(1.0)
            .with_label("Right Anchor")
            .with_range(0.0, 1.0)
            .with_page("Layout"),
    );
    m.insert(
        "rightoffset".into(),
        Param::float(0.0)
            .with_label("Right Offset")
            .with_page("Layout"),
    );
    m.insert(
        "horigin".into(),
        Param::float(0.0)
            .with_label("Horizontal Origin")
            .with_range(0.0, 1.0)
            .with_page("Layout"),
    );
    m.insert(
        "hfillweight".into(),
        Param::float(1.0)
            .with_label("Horizontal Fill Weight")
            .with_range(0.0, 8.0)
            .with_page("Layout"),
    );
    m.insert(
        "vmode".into(),
        Param::menu("fixed", &["fixed", "fill", "anchors"])
            .with_label("Vertical Mode")
            .with_page("Layout"),
    );
    m.insert(
        "bottomanchor".into(),
        Param::float(0.0)
            .with_label("Bottom Anchor")
            .with_range(0.0, 1.0)
            .with_page("Layout"),
    );
    m.insert(
        "bottomoffset".into(),
        Param::float(0.0)
            .with_label("Bottom Offset")
            .with_page("Layout"),
    );
    m.insert(
        "topanchor".into(),
        Param::float(1.0)
            .with_label("Top Anchor")
            .with_range(0.0, 1.0)
            .with_page("Layout"),
    );
    m.insert(
        "topoffset".into(),
        Param::float(0.0)
            .with_label("Top Offset")
            .with_page("Layout"),
    );
    m.insert(
        "vorigin".into(),
        Param::float(0.0)
            .with_label("Vertical Origin")
            .with_range(0.0, 1.0)
            .with_page("Layout"),
    );
    m.insert(
        "vfillweight".into(),
        Param::float(1.0)
            .with_label("Vertical Fill Weight")
            .with_range(0.0, 8.0)
            .with_page("Layout"),
    );
    m.insert(
        "alignallow".into(),
        Param::menu("allow", &["allow", "ignore", "ignoremargin"])
            .with_label("Parent Alignment")
            .with_page("Layout"),
    );
    m.insert(
        "alignorder".into(),
        Param::float(0.0)
            .with_label("Align Order")
            .with_page("Layout"),
    );
    m.insert(
        "postoffsetx".into(),
        Param::float(0.0)
            .with_label("Post Offset X")
            .with_page("Layout"),
    );
    m.insert(
        "postoffsety".into(),
        Param::float(0.0)
            .with_label("Post Offset Y")
            .with_page("Layout"),
    );
}

pub fn append_panel_page(m: &mut IndexMap<String, Param>) {
    append_panel_page_inner(m, true);
}

pub fn append_panel_page_no_label(m: &mut IndexMap<String, Param>) {
    append_panel_page_inner(m, false);
}

fn append_panel_page_inner(m: &mut IndexMap<String, Param>, include_label: bool) {
    m.insert(
        "display".into(),
        Param::bool(true).with_label("Display").with_page("Panel"),
    );
    m.insert(
        "enable".into(),
        Param::bool(true).with_label("Enable").with_page("Panel"),
    );
    if include_label {
        m.insert(
            "label".into(),
            Param::str("").with_label("Label").with_page("Panel"),
        );
    }
    m.insert(
        "clickthrough".into(),
        Param::bool(false)
            .with_label("Click Through")
            .with_page("Panel"),
    );
}

pub fn append_look_page(m: &mut IndexMap<String, Param>) {
    m.insert(
        "opacity".into(),
        Param::float(1.0)
            .with_label("Opacity")
            .with_range(0.0, 1.0)
            .with_page("Look"),
    );
    m.insert(
        "bgcolorr".into(),
        Param::float(0.22)
            .with_label("Background R")
            .with_range(0.0, 1.0)
            .with_page("Look"),
    );
    m.insert(
        "bgcolorg".into(),
        Param::float(0.24)
            .with_label("Background G")
            .with_range(0.0, 1.0)
            .with_page("Look"),
    );
    m.insert(
        "bgcolorb".into(),
        Param::float(0.30)
            .with_label("Background B")
            .with_range(0.0, 1.0)
            .with_page("Look"),
    );
    m.insert(
        "bgalpha".into(),
        Param::float(1.0)
            .with_label("Background Alpha")
            .with_range(0.0, 1.0)
            .with_page("Look"),
    );
    m.insert(
        "topfill".into(),
        Param::menu(
            "best",
            &["off", "horizontal", "vertical", "best", "native", "outside"],
        )
        .with_label("TOP Fill")
        .with_page("Look"),
    );
    m.insert(
        "composite".into(),
        Param::menu(
            "over",
            &[
                "over", "under", "inside", "outside", "add", "subtract", "multiply",
            ],
        )
        .with_label("Composite")
        .with_page("Look"),
    );
    m.insert(
        "borderover".into(),
        Param::bool(false)
            .with_label("Border Over Children")
            .with_page("Look"),
    );
}

pub fn append_container_view_page(m: &mut IndexMap<String, Param>) {
    m.insert(
        "vieww".into(),
        Param::int(400)
            .with_label("View Width")
            .with_range(64.0, 16384.0)
            .with_page("View"),
    );
    m.insert(
        "viewh".into(),
        Param::int(300)
            .with_label("View Height")
            .with_range(64.0, 16384.0)
            .with_page("View"),
    );
}

pub fn append_container_children_page(m: &mut IndexMap<String, Param>) {
    m.insert(
        "align".into(),
        Param::menu(
            "none",
            &[
                "none", "horizlr", "horizrl", "verttb", "vertbt", "gridrows", "gridcols", "nodes",
            ],
        )
        .with_label("Align")
        .with_page("Children"),
    );
    m.insert(
        "spacing".into(),
        Param::float(0.02)
            .with_label("Spacing")
            .with_range(0.0, 0.5)
            .with_page("Children"),
    );
    m.insert(
        "alignmax".into(),
        Param::float(0.0)
            .with_label("Max per Line")
            .with_range(0.0, 64.0)
            .with_page("Children"),
    );
    for (key, label) in [
        ("marginl", "Margin L"),
        ("marginr", "Margin R"),
        ("margint", "Margin T"),
        ("marginb", "Margin B"),
    ] {
        m.insert(
            key.into(),
            Param::float(0.02)
                .with_label(label)
                .with_range(0.0, 0.5)
                .with_page("Children"),
        );
    }
    m.insert(
        "justifymethod".into(),
        Param::menu("individual", &["individual", "group"])
            .with_label("Justify Method")
            .with_page("Children"),
    );
    m.insert(
        "justifyh".into(),
        Param::menu("off", &["off", "left", "center", "right"])
            .with_label("Justify Horizontal")
            .with_page("Children"),
    );
    m.insert(
        "justifyv".into(),
        Param::menu("off", &["off", "top", "center", "bottom"])
            .with_label("Justify Vertical")
            .with_page("Children"),
    );
    m.insert(
        "fit".into(),
        Param::menu("off", &["off", "horizontal", "vertical", "best"])
            .with_label("Fit")
            .with_page("Children"),
    );
    m.insert(
        "scalex".into(),
        Param::float(1.0)
            .with_label("Scale X")
            .with_range(0.0, 8.0)
            .with_page("Children"),
    );
    m.insert(
        "scaley".into(),
        Param::float(1.0)
            .with_label("Scale Y")
            .with_range(0.0, 8.0)
            .with_page("Children"),
    );
    m.insert(
        "offsetx".into(),
        Param::float(0.0)
            .with_label("Offset X")
            .with_page("Children"),
    );
    m.insert(
        "offsety".into(),
        Param::float(0.0)
            .with_label("Offset Y")
            .with_page("Children"),
    );
    m.insert(
        "crop".into(),
        Param::menu("off", &["off", "on", "never"])
            .with_label("Crop")
            .with_page("Children"),
    );
}
