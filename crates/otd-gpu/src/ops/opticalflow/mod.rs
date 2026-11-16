use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "opticalflowTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "gain" => Param::float(1.0).with_label("Vector Gain").with_range(0.0, 20.0),
        "regularization" => Param::float(0.0001).with_label("Noise Regularization").with_range(0.000001, 0.1),
        "maxmotion" => Param::float(4.0).with_label("Maximum Motion (px/frame)").with_range(0.1, 16.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "gain"),
            f(n, c, "regularization"),
            f(n, c, "maxmotion"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Optical Flow",
            family: Family::Top,
            inputs: &["current", "previous"],
            summary: "Local Lucas-Kanade motion from previous (input 2) to current (input 1). \
             RG = 0.5 + gain * motion in UV/frame; B is clamped pixel magnitude divided by maxmotion. \
             Feed a Feedback TOP targeting the source into input 2. Use matching input sizes. \
             Estimates small translations with a 5x5 window; large motion, flat areas and changing lighting are unreliable. \
             Lower the input resolution to reduce displacement in pixels. No pyramid or body tracking.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/opticalflow.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
