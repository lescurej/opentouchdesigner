use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "convolveTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "row0" => Param::xyz([0.0, 0.0, 0.0]).with_label("Kernel Row 1"),
        "row1" => Param::xyz([0.0, 1.0, 0.0]).with_label("Kernel Row 2"),
        "row2" => Param::xyz([0.0, 0.0, 0.0]).with_label("Kernel Row 3"),
        "spread" => Param::float(1.0).with_label("Spread (px)").with_range(0.0, 32.0),
        "normalize" => Param::bool(true).with_label("Normalize"),
        "scale" => Param::float(1.0).with_label("Scale").with_range(-4.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-1.0, 1.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let r0 = val(n, c, "row0").as_vec4_f32();
    let r1 = val(n, c, "row1").as_vec4_f32();
    let r2 = val(n, c, "row2").as_vec4_f32();
    [
        [r0[0], r0[1], r0[2], f(n, c, "spread")],
        [r1[0], r1[1], r1[2], f(n, c, "scale")],
        [r2[0], r2[1], r2[2], f(n, c, "offset")],
        [b(n, c, "normalize"), 0.0, 0.0, 0.0],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Convolve",
            family: Family::Top,
            inputs: &["in"],
            summary: "An arbitrary 3x3 kernel: sharpen, blur, edge detect.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/convolve.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
