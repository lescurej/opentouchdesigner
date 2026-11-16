use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "voronoiTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "scale" => Param::float(8.0).with_label("Cells").with_range(1.0, 128.0),
        "speed" => Param::float(0.4).with_label("Speed").with_range(0.0, 8.0),
        "jitter" => Param::float(1.0).with_label("Jitter").with_range(0.0, 1.0),
        "output" => Param::menu("cells", &["cells", "edges", "distance"])
            .with_label("Output"),
        "color1" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Color 1"),
        "color2" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color 2"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "scale"),
            f(n, c, "speed"),
            f(n, c, "jitter"),
            menu(n, c, "output"),
        ],
        v4(n, c, "color1"),
        v4(n, c, "color2"),
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            label: "Voronoi",
            family: Family::Top,
            inputs: &[],
            input_families: &[],
            summary: "Cellular noise, as flat cells, edges or a distance field.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/voronoi.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}