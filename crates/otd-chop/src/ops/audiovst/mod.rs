//! Audio VST CHOP — VST3 instrument/effect host (TouchDesigner-compatible).
//!
//! Full VST3 hosting is not wired in this build yet; the operator is registered
//! so projects open and audio on the main input passes through unchanged.

use otd_core::Param;
use otd_core::indexmap::IndexMap;

use crate::data::{Channel, ChopData, slice_len};

use super::{spec_animated, ChopCtx, ChopSpec};
use super::helpers::{chop_common_page_params, merge_param_maps, with_rate};

pub const AUDIO_VST: &str = "audiovstCHOP";

const STATUS: &str =
    "VST3 plugin hosting is not implemented in this build — audio passes through unchanged.";

pub(crate) fn params() -> IndexMap<String, Param> {
    let mut vst = IndexMap::new();
    vst.insert(
        "file".into(),
        Param::str("")
            .with_label("File")
            .as_file_ref()
            .with_page("VST"),
    );
    vst.insert(
        "reloadpulse".into(),
        Param::bool(false).with_label("Reload").with_page("VST"),
    );
    vst.insert(
        "loadpluginstate".into(),
        Param::bool(true)
            .with_label("Load Plugin State on Start")
            .with_page("VST"),
    );
    vst.insert(
        "alwaysontop".into(),
        Param::bool(false)
            .with_label("Plugin GUI Always on Top")
            .with_page("VST"),
    );
    vst.insert(
        "displaygui".into(),
        Param::bool(false)
            .with_label("Display Plugin GUI")
            .with_page("VST"),
    );
    vst.insert(
        "learnparms".into(),
        Param::bool(false)
            .with_label("Learn Parameters")
            .with_page("VST"),
    );
    vst.insert(
        "regularparms".into(),
        Param::bool(false)
            .with_label("Regular Parameters")
            .with_page("VST"),
    );
    vst.insert(
        "readonlyparms".into(),
        Param::bool(false)
            .with_label("Read-Only Parameters")
            .with_page("VST"),
    );
    vst.insert(
        "clearlearnedparms".into(),
        Param::bool(false)
            .with_label("Clear Learned Parameters")
            .with_page("VST"),
    );
    vst.insert(
        "callbacks".into(),
        Param::str("")
            .with_label("Callbacks DAT")
            .as_path_ref()
            .with_page("VST"),
    );
    vst.insert(
        "custombuslayout".into(),
        Param::bool(false)
            .with_label("Custom Bus Layout")
            .with_page("Bus Layout"),
    );
    vst.insert(
        "outputbuslayout".into(),
        Param::menu("auto", &["auto", "mono", "stereo"])
            .with_label("Output Bus Layout")
            .with_page("Bus Layout"),
    );
    vst.insert(
        "maininputlayout".into(),
        Param::menu("auto", &["auto", "mono", "stereo"])
            .with_label("Main Input Bus Layout")
            .with_page("Bus Layout"),
    );
    vst.insert(
        "aux0enable".into(),
        Param::bool(false)
            .with_label("Enable")
            .with_page("Bus Layout"),
    );
    vst.insert(
        "aux0layout".into(),
        Param::menu("mono", &["mono", "stereo"])
            .with_label("Bus Layout")
            .with_page("Bus Layout"),
    );
    vst.insert(
        "customplayhead".into(),
        Param::bool(false)
            .with_label("Custom Playhead")
            .with_page("Playhead"),
    );
    vst.insert(
        "timecodeop".into(),
        Param::str("")
            .with_label("Timecode Object/ CHOP/ DAT")
            .as_path_ref()
            .with_page("Playhead"),
    );
    vst.insert(
        "tempo".into(),
        Param::float(120.0)
            .with_label("Tempo (bpm)")
            .with_range(20.0, 300.0)
            .with_page("Playhead"),
    );
    vst.insert(
        "signature1".into(),
        Param::int(4)
            .with_label("Signature")
            .with_range(1.0, 32.0)
            .with_page("Playhead"),
    );
    vst.insert(
        "signature2".into(),
        Param::int(4)
            .with_label("Signature")
            .with_range(1.0, 32.0)
            .with_page("Playhead"),
    );
    let mut m = with_rate(vst);
    if let Some(p) = m.get_mut("rate") {
        *p = Param::float(48_000.0)
            .with_label("Sample Rate")
            .with_range(8_000.0, 192_000.0);
    }
    merge_param_maps(m, chop_common_page_params())
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    c.io.note(c.path, STATUS);

    let main = c.input(0);
    if main.num_channels() > 0 && main.num_samples() > 0 {
        return main.clone();
    }

    let rate = c.rate().max(8000.0);
    let n = slice_len(rate, c.time);
    ChopData::new(vec![Channel::constant("chan1", 0.0, n)], rate, true)
}

pub fn spec() -> ChopSpec {
    spec_animated(
        AUDIO_VST,
        "Audio VST",
        &["in", "in2", "in3", "in4"],
        "Loads a VST3 plugin to generate or process audio; up to four multi-channel inputs.",
        params,
        cook,
    )
}