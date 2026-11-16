use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, Param};

use super::helpers::*;
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "moviefileinTOP";
pub const MOVIE_IN: &str = TYPE_NAME;

fn params() -> IndexMap<String, Param> {
    let unit = &["indices", "frames", "seconds", "fraction"];
    params! {
        "file" => Param::str(otd_core::media::MOVIE_IN_DEFAULT_FILE)
            .with_label("File")
            .as_file_ref()
            .with_page("Play"),
        "reload" => Param::bool(false).with_label("Reload").with_page("Play"),
        "playmode" => Param::menu("sequential", &["locked", "specify", "sequential", "timecodeop"])
            .with_label("Play Mode")
            .with_page("Play"),
        "play" => Param::bool(true).with_label("Play").with_page("Play"),
        "speed" => Param::float(1.0).with_label("Speed").with_range(-10.0, 10.0).with_page("Play"),
        "cue" => Param::bool(false).with_label("Cue").with_page("Play"),
        "cuepulse" => Param::bool(false).with_label("Cue Pulse").with_page("Play"),
        "cuepoint" => Param::float(0.0).with_label("Cue Point").with_page("Play"),
        "cuepointunit" => Param::menu("indices", unit).with_label("Cue Point Unit").with_page("Play"),
        "cuebehavior" => Param::menu("repeat", &["repeat", "play"]).with_label("Cue Behavior").with_page("Play"),
        "index" => Param::float(0.0).with_label("Index").with_page("Play"),
        "indexunit" => Param::menu("indices", unit).with_label("Index Unit").with_page("Play"),
        "loopcrossfade" => Param::float(0.0).with_label("Loop Crossfade").with_range(0.0, 60.0).with_page("Play"),
        "loopcrossfadeunit" => Param::menu("seconds", unit).with_label("Loop Crossfade Unit").with_page("Play"),
        "stepsize" => Param::int(1).with_label("Step Size").with_range(1.0, 1000.0).with_page("Play"),
        "audioloop" => Param::menu("silence", &["silence", "fade", "match"]).with_label("Audio Loop").with_page("Play"),
        "imageindexing" => Param::menu("zero", &["zero", "filename"]).with_label("Image Sequence Indexing").with_page("Play"),
        "timecodemode" => Param::menu("absolute", &["absolute", "usefiletimecode"]).with_label("Timecode Mode").with_page("Play"),
        "timecodeop" => Param::str("").with_label("Timecode Object/CHOP/DAT").as_path_ref().with_page("Play"),
        "active" => Param::bool(true).with_label("Active").with_page("Play"),

        "interp" => Param::bool(false).with_label("Interpolate Frames").with_page("Image"),
        "deinterlace" => Param::menu("off", &["off", "even", "odd", "bob"]).with_label("Deinterlace").with_page("Image"),
        "precedence" => Param::menu("even", &["even", "odd"]).with_label("Field Precedence").with_page("Image"),
        "bottomhalfalpha" => Param::bool(false).with_label("Bottom Half is Alpha (AAA)").with_page("Image"),
        "premultrgbbyalpha" => Param::menu("automatic", &["off", "on", "automatic"]).with_label("Pre-Multiply RGB by Alpha").with_page("Image"),
        "inputcolorspace" => Param::menu("automatic", &[
            "automatic", "srgb", "srgblinear", "rec601ntsc", "rec709", "rec2020",
            "rec2020st2084pq", "rec2020hlg", "dcip3", "dcip3d60", "displayp3d65",
            "displayp3d65linear", "aces2065-1", "acescg", "acesproxy", "passthrough", "rec709gamma24",
        ]).with_label("Input Color Space").with_page("Image"),
        "inputreferencewhite" => Param::menu("default", &["default", "sdr", "hdr", "ui"]).with_label("Input Reference White").with_page("Image"),
        "hlgpeaknits" => Param::float(1000.0).with_label("HLG Peak Nits").with_range(100.0, 10000.0).with_page("Image"),
        "coloredbottomright" => Param::bool(true).with_label("Colored Bottom Right Square").with_page("Image"),
        "decodepixelformat" => Param::menu("fastest", &["fastest", "best"]).with_label("Decode Pixel Format").with_page("Image"),
        "resw" => Param::int(1280).with_label("Fallback W").with_range(1.0, 16384.0).with_page("Image"),
        "resh" => Param::int(720).with_label("Fallback H").with_range(1.0, 16384.0).with_page("Image"),

        "trim" => Param::bool(false).with_label("Trim").with_page("Trim"),
        "tstart" => Param::float(0.0).with_label("Trim Start").with_page("Trim"),
        "tstartunit" => Param::menu("indices", unit).with_label("Trim Start Unit").with_page("Trim"),
        "tend" => Param::float(1.0).with_label("Trim End").with_page("Trim"),
        "tendunit" => Param::menu("fraction", unit).with_label("Trim End Unit").with_page("Trim"),
        "extendleft" => Param::menu("hold", &["hold", "cycle", "mirror", "black", "zero"]).with_label("Extend Left").with_page("Trim"),
        "extendright" => Param::menu("cycle", &["hold", "cycle", "mirror", "black", "zero"]).with_label("Extend Right").with_page("Trim"),
        "overridesample" => Param::bool(false).with_label("Override Sample Rate").with_page("Trim"),
        "samplerate" => Param::float(30.0).with_label("Sample Rate").with_range(1.0, 240.0).with_page("Trim"),

        "prereadframes" => Param::int(16).with_label("Pre-Read Frames").with_range(0.0, 120.0).with_page("Tune"),
        "frametimeout" => Param::int(5000).with_label("Frame Read Timeout (ms)").with_range(0.0, 60000.0).with_page("Tune"),
        "frametimeoutstrat" => Param::menu("playback", &["keep", "playback", "seeking"]).with_label("Frame Timeout Strategy").with_page("Tune"),
        "alwaysloadinitial" => Param::bool(true).with_label("Always Load Initial Frame").with_page("Tune"),
        "opentimeout" => Param::int(10000).with_label("File Open Timeout (ms)").with_range(0.0, 60000.0).with_page("Tune"),
        "predownloadhttp" => Param::bool(false).with_label("Pre-Download HTTP Addresses").with_page("Tune"),
        "updateimage" => Param::bool(true).with_label("Update Image").with_page("Tune"),
        "maxdecodecpus" => Param::int(0).with_label("Max Decode CPUs (0=all)").with_range(0.0, 64.0).with_page("Tune"),
        "highperfread" => Param::bool(false).with_label("High Performance Read").with_page("Tune"),
        "highperfreadfactor" => Param::int(1).with_label("High Performance Read Factor").with_range(1.0, 8.0).with_page("Tune"),
        "hwdecode" => Param::bool(true).with_label("Hardware Decode").with_page("Tune"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref().with_page("Tune"),

        "outputresolution" => Param::menu("useinput", &[
            "useinput", "eighth", "quarter", "half", "2x", "4x", "8x", "fit", "limit", "custom",
        ]).with_label("Output Resolution").with_page("Common"),
        "resolutionw" => Param::int(1280).with_label("W").with_range(1.0, 16384.0).with_page("Common"),
        "resolutionh" => Param::int(720).with_label("H").with_range(1.0, 16384.0).with_page("Common"),
        "resmult" => Param::bool(true).with_label("Use Global Res Multiplier").with_page("Common"),
        "outputaspect" => Param::menu("useinput", &["useinput", "resolution", "custom"]).with_label("Output Aspect").with_page("Common"),
        "aspect1" => Param::float(1.0).with_label("Aspect1").with_page("Common"),
        "aspect2" => Param::float(1.0).with_label("Aspect2").with_page("Common"),
        "armenu" => Param::menu("16:9", &["16:9", "4:3", "1:1", "2.39:1"]).with_label("Aspect Menu").with_page("Common"),
        "filtertype" => Param::menu("linear", &["nearest", "linear", "mipmap"]).with_label("Viewer Smoothness").with_page("Common"),
        "fillmode" => Param::menu("nativeres", &[
            "useinput", "fill", "width", "height", "best", "outside", "nativeres",
        ]).with_label("Fill Viewer").with_page("Common"),
        "npasses" => Param::int(1).with_label("Passes").with_range(1.0, 32.0).with_page("Common"),
        "chanmask" => Param::str("rgba").with_label("Channel Mask").with_page("Common"),
        "format" => Param::menu("useinput", &[
            "useinput", "rgba8fixed", "srgba8fixed", "rgba16float", "rgba32float",
            "rgb10a2fixed", "rgba16fixed", "rgba11float", "rgb16float",
        ]).with_label("Pixel Format").with_page("Common"),
    }
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Movie File In",
            family: Family::Top,
            inputs: &[],
            summary: "Plays an image or a movie file.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack: pack_none,
        cook: TopCook::MovieIn,
    }
}