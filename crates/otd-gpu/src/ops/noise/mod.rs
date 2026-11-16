use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "noiseTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "period" => Param::float(0.25).with_label("Period").with_range(0.001, 64.0),
        "harmonics" => Param::int(3).with_label("Harmonics").with_range(0.0, 10.0),
        "spread" => Param::float(2.0).with_label("Harmonic Spread").with_range(1.0, 64.0),
        "roughness" => Param::float(0.5).with_label("Roughness").with_range(0.0, 8.0),
        "gain" => Param::float(1.0).with_label("Harmonic Gain").with_range(0.0, 8.0),
        "exponent" => Param::float(1.0).with_label("Exponent").with_range(0.01, 8.0),
        "seed" => Param::float(0.0).with_label("Seed"),
        "translate" => Param::xyz([0.0, 0.0, 0.0]).with_label("Translate"),
        "monochrome" => Param::bool(true).with_label("Monochrome"),
        "amplitude" => Param::float(1.0).with_label("Amplitude").with_range(0.0, 8.0),
        "offset" => Param::float(0.5).with_label("Offset").with_range(-4.0, 4.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let t = val(n, c, "translate");
    let t = t.as_vec4_f32();
    [
        [
            f(n, c, "period"),
            f(n, c, "harmonics"),
            f(n, c, "roughness"),
            f(n, c, "exponent"),
        ],
        [t[0], t[1], t[2], b(n, c, "monochrome")],
        [
            f(n, c, "amplitude"),
            f(n, c, "offset"),
            f(n, c, "seed"),
            f(n, c, "spread"),
        ],
        [f(n, c, "gain"), 0.0, 0.0, 0.0],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Noise",
            family: Family::Top,
            inputs: &[],
            summary: "Fractal value noise. Animate Translate Z to make it move.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/noise.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}