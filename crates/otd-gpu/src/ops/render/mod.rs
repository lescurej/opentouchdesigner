use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, Param};

use super::helpers::*;
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "renderTOP";
pub const RENDER: &str = TYPE_NAME;

fn params_render() -> IndexMap<String, Param> {
    with_res(params! {
        "geometry" => Param::str("").with_label("Geometry").as_path_ref(),
        "camera" => Param::str("").with_label("Camera").as_path_ref(),
        "light" => Param::str("").with_label("Light").as_path_ref(),
        "background" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Background"),
        "ambient" => Param::float(0.12).with_label("Ambient").with_range(0.0, 1.0),
        "wireframe" => Param::bool(false).with_label("Wireframe"),
        "cull" => Param::menu("back", &["back", "front", "none"]).with_label("Cull"),
        "output" => Param::menu("color", &["color", "depth"]).with_label("Output"),
        "depthnear" => Param::float(0.1).with_label("Depth Near").with_range(0.0, 1000.0),
        "depthfar" => Param::float(20.0).with_label("Depth Far").with_range(0.01, 10000.0),
    })
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Render",
            family: Family::Top,
            inputs: &[],
            summary: "Draws the 3D scene referenced by Geometry, Camera and Light.",
            time_dependent: true,
            params: params_render,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack: pack_none,
        cook: TopCook::Render,
    }
}