use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use otd_core::Value;

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "tileTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "repeat" => Param::new(Value::Vec2([2.0, 2.0])).with_label("Repeat"),
        "offset" => Param::new(Value::Vec2([0.0, 0.0])).with_label("Offset"),
        "mirror" => Param::bool(false).with_label("Mirror"),
        "rowshift" => Param::float(0.0).with_label("Row Shift").with_range(0.0, 1.0),
        "columnshift" => Param::float(0.0).with_label("Column Shift").with_range(0.0, 1.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let r = val(n, c, "repeat").as_vec4_f32();
    let o = val(n, c, "offset").as_vec4_f32();
    [
        [r[0], r[1], o[0], o[1]],
        [
            b(n, c, "mirror"),
            f(n, c, "rowshift"),
            f(n, c, "columnshift"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Tile",
            family: Family::Top,
            inputs: &["in"],
            summary: "Repeat the image across the frame, optionally mirrored.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/tile.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
