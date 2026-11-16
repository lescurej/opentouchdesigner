use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param, Value};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "rectangleTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "centre" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Centre"),
        "size" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Size"),
        "softness" => Param::float(0.0).with_label("Softness").with_range(0.0, 0.5),
        "corner" => Param::float(0.0).with_label("Corner Radius").with_range(0.0, 0.5),
        "border" => Param::float(0.0).with_label("Border Width").with_range(0.0, 0.5),
        "bordercolor" => Param::float(0.0).with_label("Border Grey").with_range(0.0, 1.0),
        "fill" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Fill"),
        "background" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Background"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let centre = val(n, c, "centre").as_vec4_f32();
    let size = val(n, c, "size").as_vec4_f32();
    [
        // Half extents: a distance field is symmetric about the centre, and
        // halving here means the shader does not do it per pixel.
        [centre[0], centre[1], size[0] * 0.5, size[1] * 0.5],
        v4(n, c, "fill"),
        v4(n, c, "background"),
        [
            f(n, c, "softness"),
            f(n, c, "corner"),
            f(n, c, "border"),
            f(n, c, "bordercolor"),
        ],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Rectangle",
            family: Family::Top,
            inputs: &[],
            summary: "A rectangle, with rounded corners and an optional border.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/rectangle.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
