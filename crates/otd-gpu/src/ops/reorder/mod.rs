use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "reorderTOP";

const REORDER_SOURCES: &[&str] = &[
    "input1 r",
    "input1 g",
    "input1 b",
    "input1 a",
    "input2 r",
    "input2 g",
    "input2 b",
    "input2 a",
    "input1 luma",
    "input2 luma",
    "one",
    "zero",
];

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "red" => Param::menu("input1 r", REORDER_SOURCES).with_label("Red"),
        "green" => Param::menu("input1 g", REORDER_SOURCES).with_label("Green"),
        "blue" => Param::menu("input1 b", REORDER_SOURCES).with_label("Blue"),
        "alpha" => Param::menu("input1 a", REORDER_SOURCES).with_label("Alpha"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "red"),
            menu(n, c, "green"),
            menu(n, c, "blue"),
            menu(n, c, "alpha"),
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
            label: "Reorder",
            family: Family::Top,
            inputs: &["in1", "in2"],
            summary: "Build each output channel from a channel of either input.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/reorder.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
