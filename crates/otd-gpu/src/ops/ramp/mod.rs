use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param, Value};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec, WideParams, widen};

pub const TYPE_NAME: &str = "rampTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "dat" => Param::str("").with_label("DAT").as_path_ref(),
        "type" => Param::menu("horizontal", &["horizontal", "vertical", "radial", "circular"]).with_label("Type"),
        "phase" => Param::float(0.0).with_label("Phase").with_range(0.0, 1.0),
        "period" => Param::float(1.0).with_label("Period").with_range(0.001, 32.0),
        "position" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Position"),
        "extendleft" => Param::menu("hold", &["hold", "repeat", "mirror", "black", "zero"]).with_label("Extend Left"),
        "extendright" => Param::menu("hold", &["hold", "repeat", "mirror", "black", "zero"]).with_label("Extend Right"),
        "interp" => Param::menu("linear", &["step", "linear", "easeineaseout", "hermite"]).with_label("Interpolate Notches"),
        "tension" => Param::float(0.0).with_label("Curve Tension").with_range(0.0, 1.0),
        "antialias" => Param::float(1.0).with_label("Anti-Alias").with_range(0.0, 4.0),
        "fitaspect" => Param::menu("fill", &["fill", "fithorz", "fitvert", "fitbest"]).with_label("Fit Aspect"),
        "color1" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Color 1"),
        "color2" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color 2"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let pos = val(n, c, "position").as_vec4_f32();
    [
        [
            menu(n, c, "type"),
            f(n, c, "phase"),
            f(n, c, "period"),
            menu(n, c, "interp"),
        ],
        v4(n, c, "color1"),
        v4(n, c, "color2"),
        [
            menu(n, c, "extendleft"),
            menu(n, c, "extendright"),
            pos[0],
            pos[1],
        ],
    ]
}

pub fn pack_wide(n: &Node, c: &EvalContext) -> WideParams {
    let mut out = widen(pack(n, c));
    out[4] = [
        f(n, c, "antialias"),
        menu(n, c, "fitaspect"),
        f(n, c, "tension"),
        0.0,
    ];
    out
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Ramp",
            family: Family::Top,
            inputs: &[],
            summary: "Gradient from a DAT table or two colour stops — horizontal, vertical, radial or circular.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/ramp.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Ramp,
    }
}