//! `geotextCOMP` — 3D text in the scene (layout params; mesh generation is partial).

use otd_core::indexmap::IndexMap;
use otd_core::{Connector, OpDef, OpRegistry, Param};

pub const GEOTEXT: &str = "geotextCOMP";

pub fn is_geotext_type(op_type: &str) -> bool {
    op_type == GEOTEXT
}

fn params_geotext() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "text".into(),
        Param::str("").with_label("Text").with_page("Text"),
    );
    m.insert(
        "scale".into(),
        Param::float(1.0)
            .with_label("Scale")
            .with_range(0.01, 200.0)
            .with_page("Transform"),
    );
    m.insert(
        "tx".into(),
        Param::float(0.0)
            .with_label("Translate X")
            .with_page("Transform"),
    );
    m.insert(
        "ty".into(),
        Param::float(0.0)
            .with_label("Translate Y")
            .with_page("Transform"),
    );
    m.insert(
        "tz".into(),
        Param::float(0.0)
            .with_label("Translate Z")
            .with_page("Transform"),
    );
    m.insert(
        "rx".into(),
        Param::float(0.0)
            .with_label("Rotate X")
            .with_page("Transform"),
    );
    m.insert(
        "ry".into(),
        Param::float(0.0)
            .with_label("Rotate Y")
            .with_page("Transform"),
    );
    m.insert(
        "rz".into(),
        Param::float(0.0)
            .with_label("Rotate Z")
            .with_page("Transform"),
    );
    m.insert(
        "aligny".into(),
        Param::menu("baseline", &["baseline", "bottom", "center", "top"])
            .with_label("Vertical Align")
            .with_page("Text"),
    );
    m.insert(
        "alignymode".into(),
        Param::menu("bbox", &["bbox", "origin"])
            .with_label("Vertical Align Mode")
            .with_page("Text"),
    );
    crate::panel_params::append_layout_page(&mut m, (0.05, 0.05, 0.2, 0.08));
    crate::panel_params::append_panel_page_no_label(&mut m);
    crate::panel_params::append_look_page(&mut m);
    m
}

pub const GEOTEXT_DEF: OpDef = OpDef {
    type_name: GEOTEXT,
    input_families: &[],
    label: "Geo Text",
    family: otd_core::Family::Comp,
    inputs: &[],
    summary: "3D text component with transform and alignment parameters (TouchDesigner geotext COMP).",
    time_dependent: false,
    params: params_geotext,
    connector: Connector::None,
};

pub fn register(registry: &mut OpRegistry) {
    registry.register(GEOTEXT_DEF);
}
