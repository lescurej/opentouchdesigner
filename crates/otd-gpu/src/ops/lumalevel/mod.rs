use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "lumalevelTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "brightness" => Param::float(1.0).with_label("Brightness").with_range(0.0, 4.0),
        "contrast" => Param::float(1.0).with_label("Contrast").with_range(0.0, 4.0),
        "gamma" => Param::float(1.0).with_label("Gamma").with_range(0.1, 4.0),
        "opacity" => Param::float(1.0).with_label("Opacity").with_range(0.0, 1.0),
        "blacklevel" => Param::float(0.0).with_label("Black Level").with_range(0.0, 1.0),
        "whitelevel" => Param::float(1.0).with_label("White Level").with_range(0.0, 1.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "brightness"),
            f(n, c, "contrast"),
            f(n, c, "gamma"),
            f(n, c, "opacity"),
        ],
        [f(n, c, "blacklevel"), f(n, c, "whitelevel"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Luma Level",
            family: Family::Top,
            inputs: &["in"],
            summary: "Brightness, contrast and gamma on luminance alone, keeping hue.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/lumalevel.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
