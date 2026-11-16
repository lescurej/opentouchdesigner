use otd_core::indexmap::IndexMap;
use otd_core::Param;
use otd_core::Crossing;
use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "sop" => Param::str("").with_label("SOP").as_path_ref(),
        "attrs" => Param::str("P")
            .with_label("Attributes (P N uv Cd)"),
    }
}

/// Geometry as channels: one sample per point, one channel per component.
///
/// This is what closes the instancing loop. A Copy SOP or a Noise SOP can
/// decide where 256 things go, and the positions come back out here as the
/// `tx`/`ty`/`tz` an instancing Geometry COMP reads — geometry driving
/// geometry, without either end knowing about the other.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let Some(points) = c.foreign(0) else {
        return ChopData::empty();
    };
    let stride = points.point_stride();
    let Crossing::Points { data, .. } = points else {
        return ChopData::empty();
    };
    if stride == 0 {
        return ChopData::empty();
    }
    let count = data.len() / stride;

    let wanted = c.s("attrs");
    let wanted: Vec<&str> = wanted.split_whitespace().collect();
    let suffix = ["x", "y", "z", "w"];

    let mut channels = Vec::new();
    for name in wanted {
        let Some((offset, width)) = points.attr(name) else {
            continue;
        };
        for comp in 0..width {
            let samples = (0..count)
                .map(|p| data[p * stride + offset + comp])
                .collect();
            let label = if width == 1 {
                name.to_string()
            } else {
                format!("{name}{}", suffix.get(comp).copied().unwrap_or("?"))
            };
            channels.push(Channel::new(label, samples));
        }
    }
    ChopData::new(channels, CONTROL_RATE, false)
}



pub const TYPE_NAME: &str = "soptochopCHOP";

pub fn spec() -> ChopSpec {
    super::converter_spec(
                TYPE_NAME,
                "SOP to CHOP",
                "Point attributes as channels, one sample per point.",
                params,
                cook,
            )
}
