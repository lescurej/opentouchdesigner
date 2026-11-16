use otd_core::indexmap::IndexMap;
use otd_core::{Crossing, Param};

use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "top" => Param::str("").with_label("TOP").as_path_ref(),
        "threshold" => Param::float(0.5).with_label("Luminance Threshold").with_range(0.0, 1.0),
        "minarea" => Param::float(0.002).with_label("Minimum Frame Fraction").with_range(0.0, 1.0),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let blob = match c.foreign(0) {
        Some(Crossing::Pixels { width, height, rgba }) => crate::tracking::largest(
            *width as usize, *height as usize, rgba, c.f("threshold"), c.f("minarea")),
        _ => crate::tracking::Blob::default(),
    };
    let present = blob.count > 0;
    let previous = c.state.prev.as_slice();
    let velocity = if present && previous.len() == 3 && previous[2] > 0.0 {
        let dt = c.time.dt.max(0.000001) as f32;
        [(blob.x - previous[0]) / dt, (blob.y - previous[1]) / dt]
    } else { [0.0, 0.0] };
    c.state.prev = vec![blob.x, blob.y, if present {1.0} else {0.0}];
    ChopData::new([
        ("present", if present {1.0} else {0.0}), ("count", blob.count as f32),
        ("x", blob.x), ("y", blob.y), ("area", blob.area),
        ("width", blob.width), ("height", blob.height),
        ("vx", velocity[0]), ("vy", velocity[1]),
    ].into_iter().map(|(name, value)| Channel::constant(name, value, 1)).collect(),
        CONTROL_RATE, false)
}


pub const TYPE_NAME: &str = "blobtrackCHOP";

pub fn spec() -> ChopSpec {
    let mut s = super::converter_spec(
        TYPE_NAME,
        "Blob Track",
        "Largest connected bright region. Channels: present, count, x, y, area, width, height, vx, vy. \
         Coordinates and sizes are 0..1, origin top-left; velocity is frame-fractions/second. \
         CPU readback of last frame; downsample to 160x90 first. \
         Not person recognition or persistent identity: switching largest regions can jump. \
         A missing/empty input returns zero channels values.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
