use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "thresholdTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "threshold" => Param::float(0.5).with_label("Threshold").with_range(0.0, 4.0),
        "softness" => Param::float(0.0).with_label("Softness").with_range(0.0, 1.0),
        "source" => Param::menu("luminance", &["luminance", "maximum", "alpha"])
            .with_label("Compare"),
        "invert" => Param::bool(false).with_label("Invert"),
        "below" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Below Colour"),
        "above" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Above Colour"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "threshold"),
            f(n, c, "softness"),
            menu(n, c, "source"),
            b(n, c, "invert"),
        ],
        v4(n, c, "below"),
        v4(n, c, "above"),
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Threshold",
            family: Family::Top,
            inputs: &["in"],
            summary: "Split the image in two at a level, with a soft edge.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/threshold.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
