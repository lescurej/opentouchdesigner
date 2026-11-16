//! `textCOMP` — panel text drawn via an interior `textTOP` → `outTOP` chain.

use otd_core::indexmap::IndexMap;
use otd_core::{Connector, OpDef, OpRegistry, Param};

pub const TEXT: &str = "textCOMP";

pub fn is_text_type(op_type: &str) -> bool {
    op_type == TEXT
}

fn params_text_comp() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "text".into(),
        Param::str("").with_label("Text").with_page("Text"),
    );
    m.insert(
        "fontsize".into(),
        Param::float(0.5)
            .with_label("Font Size")
            .with_range(0.05, 2.0)
            .with_page("Text"),
    );
    m.insert(
        "halign".into(),
        Param::menu("left", &["left", "centre", "right"])
            .with_label("Horizontal Align")
            .with_page("Text"),
    );
    m.insert(
        "valign".into(),
        Param::menu("centre", &["top", "centre", "bottom"])
            .with_label("Vertical Align")
            .with_page("Text"),
    );
    m.insert(
        "wordwrap".into(),
        Param::bool(true).with_label("Word Wrap").with_page("Text"),
    );
    m.insert(
        "editmode".into(),
        Param::menu("locked", &["locked", "editable", "selectonly"])
            .with_label("Edit Mode")
            .with_page("Text"),
    );
    crate::panel_params::append_layout_page(&mut m, (0.05, 0.05, 0.2, 0.08));
    crate::panel_params::append_panel_page_no_label(&mut m);
    crate::panel_params::append_look_page(&mut m);
    m
}

pub const TEXT_DEF: OpDef = OpDef {
    type_name: TEXT,
    input_families: &[],
    label: "Text",
    family: otd_core::Family::Comp,
    inputs: &[],
    summary: "Displays formatted text in a panel rect; interior textTOP reads parent Text parameters.",
    time_dependent: false,
    params: params_text_comp,
    connector: Connector::None,
};

pub fn register(registry: &mut OpRegistry) {
    registry.register(TEXT_DEF);
}
