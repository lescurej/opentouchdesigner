use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "textTOP";
pub const TEXT: &str = TYPE_NAME;

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "field" => Param::str("").with_label("Field Component").as_path_ref(),
        "dat" => Param::str("").with_label("DAT").as_path_ref(),
        "text" => Param::str("OpenTouchDesigner").with_label("Text"),
        // A file reference, so a bundle copies the font with the project. A
        // show that opens on the wrong machine with the wrong face is the
        // whole reason bundles exist.
        "font" => Param::str("").with_label("Font File").as_file_ref(),
        "size" => Param::float(48.0).with_label("Size (px)").with_range(4.0, 512.0),
        "linespacing" => Param::float(1.2).with_label("Line Spacing").with_range(0.5, 4.0),
        "halign" => Param::menu("centre", &["left", "centre", "right"])
            .with_label("Horizontal Align"),
        "valign" => Param::menu("centre", &["top", "centre", "bottom"])
            .with_label("Vertical Align"),
        "wrap" => Param::bool(true).with_label("Word Wrap"),
        "color" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color"),
        "background" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Background"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        v4(n, c, "color"),
        v4(n, c, "background"),
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Text",
            family: Family::Top,
            inputs: &[],
            summary: "Draws text, using a font file or the system's.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/text.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Text,
    }
}
