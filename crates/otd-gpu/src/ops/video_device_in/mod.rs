use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "videodeviceinTOP";
pub const VIDEO_DEVICE_IN: &str = TYPE_NAME;

fn params() -> IndexMap<String, Param> {
    params! {
        "device" => Param::str("").with_label("Device (blank = default)"),
        // Requested, not commanded: a camera only does the modes it does, so
        // these are negotiated to the nearest one the device reports.
        "resw" => Param::int(1280).with_label("Requested W").with_range(1.0, 16384.0),
        "resh" => Param::int(720).with_label("Requested H").with_range(1.0, 16384.0),
        "fps" => Param::float(30.0).with_label("Requested Frame Rate").with_range(1.0, 240.0),
        "active" => Param::bool(true).with_label("Active"),
    }
}


fn pack(_n: &Node, _c: &EvalContext) -> PackedParams {
    [[0.0; 4]; 4]
}


pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Video Device In",
            family: Family::Top,
            inputs: &[],
            summary: "Frames from a camera or capture device.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::VideoIn,
    }
}
