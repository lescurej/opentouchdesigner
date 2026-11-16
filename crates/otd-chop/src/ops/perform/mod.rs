use otd_core::indexmap::IndexMap;

use crate::data::{Channel, ChopData, slice_len};

use super::helpers::with_rate;
use super::{spec_animated, ChopCtx, ChopSpec};

pub const TYPE_NAME: &str = "performCHOP";
pub const PERFORM: &str = TYPE_NAME;

pub(crate) fn params() -> IndexMap<String, otd_core::Param> {
    with_rate(IndexMap::new())
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let p = c.io.perform;
    let channels = vec![
        Channel::constant("cookrate", p.cookrate as f32, n),
        Channel::constant("fps", p.fps as f32, n),
        Channel::constant("msec", p.msec as f32, n),
        Channel::constant("timeslicestep", p.timeslicestep as f32, n),
        Channel::constant("timeslicemsec", p.timeslicemsec as f32, n),
    ];
    ChopData::new(channels, rate, true)
}

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Perform",
        &[],
        "Cook rate, frame time, and time-slice size (TouchDesigner Perform CHOP).",
        params,
        cook,
    )
}