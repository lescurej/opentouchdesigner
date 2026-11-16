use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "hsvadjustTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "hue" => Param::float(0.0).with_label("Hue Shift").with_range(-0.5, 0.5),
        "saturation" => Param::float(1.0).with_label("Saturation").with_range(0.0, 4.0),
        "value" => Param::float(1.0).with_label("Value").with_range(0.0, 4.0),
        "contrast" => Param::float(1.0).with_label("Contrast").with_range(0.0, 4.0),
        "rangecentre" => Param::float(0.0).with_label("Range Centre").with_range(0.0, 1.0),
        "rangewidth" => Param::float(1.0).with_label("Range Width").with_range(0.0, 1.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "hue"),
            f(n, c, "saturation"),
            f(n, c, "value"),
            f(n, c, "contrast"),
        ],
        [f(n, c, "rangecentre"), f(n, c, "rangewidth"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "HSV Adjust",
            family: Family::Top,
            inputs: &["in"],
            summary: "Hue, saturation and value, optionally over one band of the wheel.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/hsv.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
