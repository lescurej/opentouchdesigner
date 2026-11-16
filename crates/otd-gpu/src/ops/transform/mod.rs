use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param, Value};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "transformTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "translate" => Param::new(Value::Vec2([0.0, 0.0])).with_label("Translate"),
        "rotate" => Param::float(0.0).with_label("Rotate").with_range(-180.0, 180.0),
        "scale" => Param::new(Value::Vec2([1.0, 1.0])).with_label("Scale"),
        "extend" => Param::menu("zero", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let t = val(n, c, "translate").as_vec4_f32();
    let s = val(n, c, "scale").as_vec4_f32();
    [
        [t[0], t[1], f(n, c, "rotate").to_radians(), 0.0],
        [s[0], s[1], menu(n, c, "extend"), 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Transform",
            family: Family::Top,
            inputs: &["in"],
            summary: "Translate, rotate and scale in UV space.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/transform.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}