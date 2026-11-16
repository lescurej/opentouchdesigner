use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param, Value};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "circleTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "centre" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Centre"),
        "radius" => Param::new(Value::Vec2([0.3, 0.3])).with_label("Radius"),
        "softness" => Param::float(0.0).with_label("Softness").with_range(0.0, 1.0),
        "aspect" => Param::bool(true).with_label("Correct Aspect"),
        "fill" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Fill"),
        "background" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Background"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let centre = val(n, c, "centre").as_vec4_f32();
    let radius = val(n, c, "radius").as_vec4_f32();
    [
        [centre[0], centre[1], radius[0], radius[1]],
        v4(n, c, "fill"),
        v4(n, c, "background"),
        [f(n, c, "softness"), b(n, c, "aspect"), 0.0, 0.0],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Circle",
            family: Family::Top,
            inputs: &[],
            summary: "An antialiased disc or ellipse.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/circle.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
