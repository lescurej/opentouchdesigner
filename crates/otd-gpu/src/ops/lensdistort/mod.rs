use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use otd_core::Value;

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "lensdistortTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "k1" => Param::float(0.0).with_label("Distort (- bulges, + corrects)").with_range(-1.0, 1.0),
        "k2" => Param::float(0.0).with_label("Distort Squared").with_range(-1.0, 1.0),
        "scale" => Param::float(1.0).with_label("Scale").with_range(0.1, 4.0),
        "center" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Center"),
        "extend" => Param::menu("zero", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let ctr = val(n, c, "center").as_vec4_f32();
    [
        [
            f(n, c, "k1"),
            f(n, c, "k2"),
            f(n, c, "scale"),
            menu(n, c, "extend"),
        ],
        [ctr[0], ctr[1], 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Lens Distort",
            family: Family::Top,
            inputs: &["in"],
            summary: "Bulge or pinch the frame. Negative Distort bulges (fisheye), \
             positive corrects a bulge.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/lensdistort.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
