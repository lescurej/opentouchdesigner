use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use otd_core::Value;

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "cornerpinTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "bottomleft" => Param::new(Value::Vec2([0.0, 0.0])).with_label("Bottom Left"),
        "bottomright" => Param::new(Value::Vec2([1.0, 0.0])).with_label("Bottom Right"),
        "topleft" => Param::new(Value::Vec2([0.0, 1.0])).with_label("Top Left"),
        "topright" => Param::new(Value::Vec2([1.0, 1.0])).with_label("Top Right"),
        "extend" => Param::menu("zero", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let bl = val(n, c, "bottomleft").as_vec4_f32();
    let br = val(n, c, "bottomright").as_vec4_f32();
    let tl = val(n, c, "topleft").as_vec4_f32();
    let tr = val(n, c, "topright").as_vec4_f32();
    [
        [bl[0], bl[1], br[0], br[1]],
        [tl[0], tl[1], tr[0], tr[1]],
        [menu(n, c, "extend"), 0.0, 0.0, 0.0],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Corner Pin",
            family: Family::Top,
            inputs: &["in"],
            summary: "Drag the image's four corners anywhere — a perspective warp.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/cornerpin.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
