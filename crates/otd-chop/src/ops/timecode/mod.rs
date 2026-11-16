use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{Channel, ChopData, slice_len};
use crate::ops::ChopCtx;
use crate::ops::helpers::with_rate;

use super::{spec_animated, ChopSpec};
use crate::timecode::{self, Smpte};

pub(crate) fn params() -> IndexMap<String, Param> {
    with_rate(params! {
        "ratemenu" => Param::menu("60", &["24", "25", "30", "60"]).with_label("FPS"),
        "dropframe" => Param::bool(false).with_label("Drop Frame"),
        "source" => Param::menu("timeline", &["timeline", "abs"]).with_label("Source"),
    })
}

fn fps_from_menu(menu: usize) -> f64 {
    match menu {
        0 => 24.0,
        1 => 25.0,
        2 => 30.0,
        _ => 60.0,
    }
}

pub(crate) fn smpte_channels(smpte: &Smpte, n: usize) -> Vec<Channel> {
    let names = [
        ("hour", smpte.hour as f32),
        ("min", smpte.min as f32),
        ("sec", smpte.sec as f32),
        ("frame", smpte.frame as f32),
        ("frame_frac", smpte.frame_frac as f32),
        ("total_frames", smpte.total_frames as f32),
    ];
    names
        .into_iter()
        .map(|(name, v)| Channel::constant(name, v, n))
        .collect()
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let fps = fps_from_menu(c.menu("ratemenu"));
    let drop = c.b("dropframe");
    let t = if c.menu("source") == 0 {
        c.time.abs_time
    } else {
        c.time.time
    };
    let smpte = timecode::smpte_from_seconds(t, fps, drop);
    ChopData::new(smpte_channels(&smpte, n), rate, true)
}

pub const TYPE_NAME: &str = "timecodeCHOP";

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Timecode",
        &[],
        "Hour, minute, second and frame from the timeline or absolute time.",
        params,
        cook,
    )
}
