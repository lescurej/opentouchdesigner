use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param, Value};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "mirrorTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "mode" => Param::menu("horizontal", &["horizontal", "vertical", "quad", "radial"])
            .with_label("Mode"),
        "segments" => Param::float(6.0).with_label("Segments").with_range(2.0, 32.0),
        "angle" => Param::float(0.0).with_label("Angle").with_range(-180.0, 180.0),
        "centre" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Centre"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let centre = val(n, c, "centre").as_vec4_f32();
    [
        [
            menu(n, c, "mode"),
            f(n, c, "segments"),
            f(n, c, "angle").to_radians(),
            centre[0],
        ],
        [centre[1], 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Mirror",
            family: Family::Top,
            inputs: &["in"],
            summary: "Fold the image onto itself, including a radial kaleidoscope.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/mirror.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
