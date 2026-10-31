use otd_core::indexmap::IndexMap;
use otd_core::{EvalContext, Node, Param, Value};

use crate::isf;

use super::helpers::*;
use super::{PackedParams, WideParams, NO_PARAMS, widen};

// ------------------------------------------------------------- constant TOP

pub(crate) fn params_constant() -> IndexMap<String, Param> {
    with_res(params! {
        "color" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Color"),
    })
}

pub(crate) fn pack_constant(n: &Node, c: &EvalContext) -> PackedParams {
    [v4(n, c, "color"), [0.0; 4], [0.0; 4], [0.0; 4]]
}

// ---------------------------------------------------------------- noise TOP

pub(crate) fn params_noise() -> IndexMap<String, Param> {
    with_res(params! {
        "period" => Param::float(0.25).with_label("Period").with_range(0.01, 4.0),
        "harmonics" => Param::int(3).with_label("Harmonics").with_range(1.0, 10.0),
        "roughness" => Param::float(0.5).with_label("Roughness").with_range(0.0, 1.0),
        "exponent" => Param::float(1.0).with_label("Exponent").with_range(0.1, 8.0),
        "translate" => Param::xyz([0.0, 0.0, 0.0]).with_label("Translate"),
        "monochrome" => Param::bool(true).with_label("Monochrome"),
        "amplitude" => Param::float(1.0).with_label("Amplitude").with_range(0.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-1.0, 1.0),
    })
}

pub(crate) fn pack_noise(n: &Node, c: &EvalContext) -> PackedParams {
    let t = val(n, c, "translate");
    let t = t.as_vec4_f32();
    [
        [
            f(n, c, "period"),
            f(n, c, "harmonics"),
            f(n, c, "roughness"),
            f(n, c, "exponent"),
        ],
        [t[0], t[1], t[2], b(n, c, "monochrome")],
        [f(n, c, "amplitude"), f(n, c, "offset"), 0.0, 0.0],
        [0.0; 4],
    ]
}

// ----------------------------------------------------------------- ramp TOP

pub(crate) fn params_ramp() -> IndexMap<String, Param> {
    with_res(params! {
        "type" => Param::menu("horizontal", &["horizontal", "vertical", "radial"]).with_label("Type"),
        "phase" => Param::float(0.0).with_label("Phase").with_range(0.0, 1.0),
        "color1" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Color 1"),
        "color2" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color 2"),
    })
}

pub(crate) fn pack_ramp(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "type"), f(n, c, "phase"), 0.0, 0.0],
        v4(n, c, "color1"),
        v4(n, c, "color2"),
        [0.0; 4],
    ]
}

// ---------------------------------------------------------------- level TOP

pub(crate) fn params_level() -> IndexMap<String, Param> {
    params! {
        "brightness" => Param::float(1.0).with_label("Brightness").with_range(0.0, 4.0),
        "contrast" => Param::float(1.0).with_label("Contrast").with_range(0.0, 4.0),
        "gamma" => Param::float(1.0).with_label("Gamma").with_range(0.1, 4.0),
        "opacity" => Param::float(1.0).with_label("Opacity").with_range(0.0, 1.0),
        "blacklevel" => Param::float(0.0).with_label("Black Level").with_range(0.0, 1.0),
        "whitelevel" => Param::float(1.0).with_label("White Level").with_range(0.0, 1.0),
        "invert" => Param::float(0.0).with_label("Invert").with_range(0.0, 1.0),
    }
}

pub(crate) fn pack_level(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "brightness"),
            f(n, c, "contrast"),
            f(n, c, "gamma"),
            f(n, c, "opacity"),
        ],
        [
            f(n, c, "blacklevel"),
            f(n, c, "whitelevel"),
            f(n, c, "invert"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
    ]
}

// ------------------------------------------------------------ transform TOP

pub(crate) fn params_transform() -> IndexMap<String, Param> {
    params! {
        "translate" => Param::new(Value::Vec2([0.0, 0.0])).with_label("Translate"),
        "rotate" => Param::float(0.0).with_label("Rotate").with_range(-180.0, 180.0),
        "scale" => Param::new(Value::Vec2([1.0, 1.0])).with_label("Scale"),
        "extend" => Param::menu("zero", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
    }
}

pub(crate) fn pack_transform(n: &Node, c: &EvalContext) -> PackedParams {
    let t = val(n, c, "translate").as_vec4_f32();
    let s = val(n, c, "scale").as_vec4_f32();
    [
        [t[0], t[1], f(n, c, "rotate").to_radians(), 0.0],
        [s[0], s[1], menu(n, c, "extend"), 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

// ----------------------------------------------------------------- blur TOP

pub(crate) fn params_blur() -> IndexMap<String, Param> {
    params! {
        "size" => Param::float(8.0).with_label("Size (px)").with_range(0.0, 128.0),
    }
}

pub(crate) fn pack_blur(n: &Node, c: &EvalContext) -> PackedParams {
    // .y is the pass direction, filled in by the engine on the second pass.
    [
        [f(n, c, "size"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

// ------------------------------------------------------------ composite TOP

pub(crate) fn params_composite() -> IndexMap<String, Param> {
    params! {
        "operation" => Param::menu(
            "over",
            &[
                "over", "add", "multiply", "screen", "difference", "subtract", "maximum",
                "minimum", "under", "inside", "outside", "cross",
            ],
        ).with_label("Operation"),
        "opacity" => Param::float(1.0).with_label("Opacity").with_range(0.0, 1.0),
    }
}

pub(crate) fn pack_composite(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "operation"), f(n, c, "opacity"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

// --------------------------------------------------------------- switch TOP

pub(crate) fn params_switch() -> IndexMap<String, Param> {
    params! {
        "index" => Param::float(0.0).with_label("Index").with_range(0.0, 1.0),
        "blend" => Param::bool(false).with_label("Blend"),
    }
}

pub(crate) fn pack_switch(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [f(n, c, "index"), b(n, c, "blend"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

// ------------------------------------------------------------ threshold TOP

pub(crate) fn params_threshold() -> IndexMap<String, Param> {
    params! {
        "threshold" => Param::float(0.5).with_label("Threshold").with_range(0.0, 1.0),
        "softness" => Param::float(0.02).with_label("Softness").with_range(0.0, 1.0),
        "source" => Param::menu("luminance", &["luminance", "maximum", "alpha"])
            .with_label("Compare"),
        "invert" => Param::bool(false).with_label("Invert"),
        "below" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Below Colour"),
        "above" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Above Colour"),
    }
}

pub(crate) fn pack_threshold(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "threshold"),
            f(n, c, "softness"),
            menu(n, c, "source"),
            b(n, c, "invert"),
        ],
        v4(n, c, "below"),
        v4(n, c, "above"),
        [0.0; 4],
    ]
}

// ----------------------------------------------------------------- edge TOP

pub(crate) fn params_edge() -> IndexMap<String, Param> {
    params! {
        "strength" => Param::float(1.0).with_label("Strength").with_range(0.0, 8.0),
        "width" => Param::float(1.0).with_label("Width (px)").with_range(0.0, 16.0),
        "direction" => Param::menu("both", &["both", "horizontal", "vertical"])
            .with_label("Direction"),
        "keepcolor" => Param::float(0.0).with_label("Keep Colour").with_range(0.0, 1.0),
        "color" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Edge Colour"),
    }
}

pub(crate) fn pack_edge(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "strength"),
            f(n, c, "width"),
            menu(n, c, "direction"),
            f(n, c, "keepcolor"),
        ],
        v4(n, c, "color"),
        [0.0; 4],
        [0.0; 4],
    ]
}

// ------------------------------------------------------------------ hsv TOP

pub(crate) fn params_hsv() -> IndexMap<String, Param> {
    params! {
        "hue" => Param::float(0.0).with_label("Hue Shift").with_range(-0.5, 0.5),
        "saturation" => Param::float(1.0).with_label("Saturation").with_range(0.0, 4.0),
        "value" => Param::float(1.0).with_label("Value").with_range(0.0, 4.0),
        "contrast" => Param::float(1.0).with_label("Contrast").with_range(0.0, 4.0),
        "rangecentre" => Param::float(0.0).with_label("Range Centre").with_range(0.0, 1.0),
        "rangewidth" => Param::float(1.0).with_label("Range Width").with_range(0.0, 1.0),
    }
}

pub(crate) fn pack_hsv(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "hue"),
            f(n, c, "saturation"),
            f(n, c, "value"),
            f(n, c, "contrast"),
        ],
        [f(n, c, "rangecentre"), f(n, c, "rangewidth"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

// ----------------------------------------------------------------- flip TOP

pub(crate) fn params_flip() -> IndexMap<String, Param> {
    params! {
        "flipx" => Param::bool(true).with_label("Flip Horizontally"),
        "flipy" => Param::bool(false).with_label("Flip Vertically"),
        "transpose" => Param::bool(false).with_label("Transpose"),
    }
}

pub(crate) fn pack_flip(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            b(n, c, "flipx"),
            b(n, c, "flipy"),
            b(n, c, "transpose"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

// --------------------------------------------------------------- mirror TOP

pub(crate) fn params_mirror() -> IndexMap<String, Param> {
    params! {
        "mode" => Param::menu("horizontal", &["horizontal", "vertical", "quad", "radial"])
            .with_label("Mode"),
        "segments" => Param::float(6.0).with_label("Segments").with_range(2.0, 32.0),
        "angle" => Param::float(0.0).with_label("Angle").with_range(-180.0, 180.0),
        "centre" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Centre"),
    }
}

pub(crate) fn pack_mirror(n: &Node, c: &EvalContext) -> PackedParams {
    let centre = val(n, c, "centre").as_vec4_f32();
    [
        [
            menu(n, c, "mode"),
            f(n, c, "segments"),
            f(n, c, "angle").to_radians(),
            centre[0],
        ],
        [centre[1], 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

// --------------------------------------------------------------- lookup TOP

pub(crate) fn params_lookup() -> IndexMap<String, Param> {
    params! {
        "source" => Param::menu("luminance", &["luminance", "red", "green", "blue", "alpha"])
            .with_label("Index By"),
    }
}

pub(crate) fn pack_lookup(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "source"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

// --------------------------------------------------------------- circle TOP

pub(crate) fn params_circle() -> IndexMap<String, Param> {
    with_res(params! {
        "centre" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Centre"),
        "radius" => Param::new(Value::Vec2([0.3, 0.3])).with_label("Radius"),
        "softness" => Param::float(0.0).with_label("Softness").with_range(0.0, 1.0),
        "aspect" => Param::bool(true).with_label("Correct Aspect"),
        "fill" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Fill"),
        "background" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Background"),
    })
}

pub(crate) fn pack_circle(n: &Node, c: &EvalContext) -> PackedParams {
    let centre = val(n, c, "centre").as_vec4_f32();
    let radius = val(n, c, "radius").as_vec4_f32();
    [
        [centre[0], centre[1], radius[0], radius[1]],
        v4(n, c, "fill"),
        v4(n, c, "background"),
        [f(n, c, "softness"), b(n, c, "aspect"), 0.0, 0.0],
    ]
}

// ------------------------------------------------------------ rectangle TOP

pub(crate) fn params_rectangle() -> IndexMap<String, Param> {
    with_res(params! {
        "centre" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Centre"),
        "size" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Size"),
        "softness" => Param::float(0.0).with_label("Softness").with_range(0.0, 0.5),
        "corner" => Param::float(0.0).with_label("Corner Radius").with_range(0.0, 0.5),
        "border" => Param::float(0.0).with_label("Border Width").with_range(0.0, 0.5),
        "bordercolor" => Param::float(0.0).with_label("Border Grey").with_range(0.0, 1.0),
        "fill" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Fill"),
        "background" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Background"),
    })
}

pub(crate) fn pack_rectangle(n: &Node, c: &EvalContext) -> PackedParams {
    let centre = val(n, c, "centre").as_vec4_f32();
    let size = val(n, c, "size").as_vec4_f32();
    [
        // Half extents: a distance field is symmetric about the centre, and
        // halving here means the shader does not do it per pixel.
        [centre[0], centre[1], size[0] * 0.5, size[1] * 0.5],
        v4(n, c, "fill"),
        v4(n, c, "background"),
        [
            f(n, c, "softness"),
            f(n, c, "corner"),
            f(n, c, "border"),
            f(n, c, "bordercolor"),
        ],
    ]
}

// ------------------------------------------------------------ chroma key TOP

pub(crate) fn params_chromakey() -> IndexMap<String, Param> {
    params! {
        "key" => Param::rgba([0.0, 1.0, 0.0, 1.0]).with_label("Key Colour"),
        "tolerance" => Param::float(0.15).with_label("Tolerance").with_range(0.0, 1.0),
        "softness" => Param::float(0.1).with_label("Softness").with_range(0.0, 1.0),
        "despill" => Param::float(1.0).with_label("Despill").with_range(0.0, 1.0),
        "replace" => Param::bool(false).with_label("Replace Rather Than Cut"),
        "replacement" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Replacement"),
    }
}

pub(crate) fn pack_chromakey(n: &Node, c: &EvalContext) -> PackedParams {
    let key = v4(n, c, "key");
    [
        [key[0], key[1], key[2], f(n, c, "tolerance")],
        [
            f(n, c, "softness"),
            f(n, c, "despill"),
            b(n, c, "replace"),
            0.0,
        ],
        v4(n, c, "replacement"),
        [0.0; 4],
    ]
}

// ----------------------------------------------------------------- math TOP

pub(crate) fn params_math() -> IndexMap<String, Param> {
    params! {
        "operation" => Param::menu(
            "add",
            &["add", "subtract", "multiply", "divide", "minimum", "maximum", "difference", "power"],
        ).with_label("Operation"),
        "gain1" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Input 1 Gain"),
        "gain2" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Input 2 Gain"),
        "offset" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Offset"),
    }
}

pub(crate) fn pack_math(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "operation"), 0.0, 0.0, 0.0],
        v4(n, c, "gain1"),
        v4(n, c, "gain2"),
        v4(n, c, "offset"),
    ]
}

// ----------------------------------------------------- null / feedback TOPs

pub(crate) fn params_none() -> IndexMap<String, Param> {
    params! {}
}

pub(crate) fn params_window_comp() -> IndexMap<String, Param> {
    params! {
        "winop" => Param::str("").with_label("Operator").as_path_ref().with_page("Window"),
        "title" => Param::str("Window").with_label("Title").with_page("Window"),
        "justifyoffsetto" => Param::menu("primarydisplay", &["primarydisplay", "specifydisplay", "alldisplays"])
            .with_label("Justify and Offset To")
            .with_page("Window"),
        "display" => Param::int(0).with_label("Display").with_range(0.0, 16.0).with_page("Window"),
        "justifyh" => Param::menu("center", &["left", "center", "right", "mouse"])
            .with_label("Justify Horizontal")
            .with_page("Window"),
        "justifyv" => Param::menu("center", &["top", "center", "bottom", "mouse"])
            .with_label("Justify Vertical")
            .with_page("Window"),
        "winoffsetx" => Param::int(0).with_label("Offset X").with_range(-8192.0, 8192.0).with_page("Window"),
        "winoffsety" => Param::int(0).with_label("Offset Y").with_range(-8192.0, 8192.0).with_page("Window"),
        "openingsize" => Param::menu("custom", &["automatic", "fill", "custom", "exclusive"])
            .with_label("Opening Size")
            .with_page("Window"),
        "winw" => Param::int(1280).with_label("Width").with_range(64.0, 16384.0).with_page("Window"),
        "winh" => Param::int(720).with_label("Height").with_range(64.0, 16384.0).with_page("Window"),
        "borders" => Param::bool(true).with_label("Borders").with_page("Window"),
        "alwaysontop" => Param::bool(false).with_label("Always on Top").with_page("Window"),
        "stretch" => Param::menu("fit", &["fit", "fill", "stretch", "native"])
            .with_label("Stretch")
            .with_page("Window"),
        "cursorvisible" => Param::bool(true).with_label("Cursor Visible").with_page("Window"),
        "allowminimize" => Param::bool(true).with_label("Allow Minimize").with_page("Window"),
        "vsyncmode" => Param::menu("enabled", &["disabled", "enabled", "halfmonitorrate"])
            .with_label("V-Sync Mode")
            .with_page("Window"),
        "open" => Param::bool(false).with_label("Open").with_page("Open/Close"),
        "performance" => Param::bool(false).with_label("Open as Perform Window").with_page("Open/Close"),
        "closeescape" => Param::bool(true).with_label("Close on Escape").with_page("Open/Close"),
        "blocksleep" => Param::bool(false).with_label("Prevent Display Sleep").with_page("Open/Close"),
    }
}

pub(crate) fn pack_none(_n: &Node, _c: &EvalContext) -> PackedParams {
    [[0.0; 4]; 4]
}

pub(crate) fn params_feedback() -> IndexMap<String, Param> {
    params! {
        "target" => Param::str("").with_label("Target TOP"),
    }
}

/// The operator type name of the Feedback TOP, special-cased by the engine.
pub const FEEDBACK: &str = "feedbackTOP";
/// Used for the Feedback blit and for any pass that is a straight copy.
pub const NULL: &str = "nullTOP";
/// Reads another TOP's output *this* frame, unlike Feedback.
pub const SELECT: &str = "selectTOP";
/// A component's texture input, surfaced as a connector on its node.
pub const IN: &str = "inTOP";
/// A component's texture output.
pub const OUT: &str = "outTOP";
/// Freezes its input on demand.
pub const CACHE: &str = "cacheTOP";
/// Compiles a shader from a parameter at cook time.
pub const GLSL: &str = "glslTOP";
/// Draws the 3D scene. Special-cased by the engine, which owns the pipeline.
pub const RENDER: &str = "renderTOP";
/// Plays an image or a movie. Special-cased by the engine, which owns the
/// decoder threads and the upload.
pub const MOVIE_IN: &str = "moviefileinTOP";
/// A camera. Same decoder, a different ffmpeg input.
pub const VIDEO_DEVICE_IN: &str = "videodeviceinTOP";
/// Channels as pixels. Special-cased by the engine, which owns the upload.
pub const CHOP_TO_TOP: &str = "choptotopTOP";
/// Draws text. Special-cased by the engine, which owns the font cache and
/// the upload.
pub const TEXT: &str = "textTOP";

pub(crate) fn params_text() -> IndexMap<String, Param> {
    with_res(params! {
        "text" => Param::str("OpenTouchDesigner").with_label("Text"),
        // A file reference, so a bundle copies the font with the project. A
        // show that opens on the wrong machine with the wrong face is the
        // whole reason bundles exist.
        "font" => Param::str("").with_label("Font File").as_file_ref(),
        "size" => Param::float(48.0).with_label("Size (px)").with_range(4.0, 512.0),
        "linespacing" => Param::float(1.2).with_label("Line Spacing").with_range(0.5, 4.0),
        "halign" => Param::menu("centre", &["left", "centre", "right"])
            .with_label("Horizontal Align"),
        "valign" => Param::menu("centre", &["top", "centre", "bottom"])
            .with_label("Vertical Align"),
        "wrap" => Param::bool(true).with_label("Word Wrap"),
        "color" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color"),
        "background" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Background"),
    })
}

pub(crate) fn pack_text(n: &Node, c: &EvalContext) -> PackedParams {
    [
        v4(n, c, "color"),
        v4(n, c, "background"),
        [0.0; 4],
        [0.0; 4],
    ]
}

/// Records its input to a movie file. Special-cased by the engine, which owns
/// the readback and the encoder subprocess.
pub const MOVIE_OUT: &str = "moviefileoutTOP";

pub(crate) fn params_movie_out() -> IndexMap<String, Param> {
    let unit = &["indices", "frames", "seconds"];
    params! {
        "type" => Param::menu("movie", &["movie", "image", "imagesequence", "stopframemovie"])
            .with_label("Type")
            .with_page("Movie Out"),
        "codec" => Param::menu("h264", &["h264", "h265", "prores"])
            .with_label("Video Codec")
            .with_page("Movie Out"),
        "imagefiletype" => Param::menu("png", &["png", "jpeg", "tiff", "bmp"])
            .with_label("Image File Type")
            .with_page("Movie Out"),
        "uniquesuff" => Param::bool(false).with_label("Unique Suffix").with_page("Movie Out"),
        "n" => Param::int(1).with_label("N").with_range(0.0, 1_000_000.0).with_page("Movie Out"),
        "leadingzerosdigits" => Param::int(4).with_label("Leading Zeros Digits").with_range(1.0, 12.0).with_page("Movie Out"),
        "file" => Param::str("out.mp4").with_label("File").with_page("Movie Out"),
        "fps" => Param::float(60.0).with_label("Movie FPS").with_range(1.0, 240.0).with_page("Movie Out"),
        "quality" => Param::int(75).with_label("Quality").with_range(0.0, 100.0).with_page("Movie Out"),
        "limitlength" => Param::bool(false).with_label("Limit Length").with_page("Movie Out"),
        "length" => Param::float(600.0).with_label("Length").with_range(0.0, 1_000_000.0).with_page("Movie Out"),
        "lengthunit" => Param::menu("frames", unit).with_label("Length Unit").with_page("Movie Out"),
        "record" => Param::bool(false).with_label("Record").with_page("Movie Out"),
        "pause" => Param::bool(false).with_label("Pause").with_page("Movie Out"),
        "addframe" => Param::bool(false).with_label("Add Frame").with_page("Movie Out"),

        "outputresolution" => Param::menu("useinput", &[
            "useinput", "eighth", "quarter", "half", "2x", "4x", "8x", "fit", "limit", "custom",
        ]).with_label("Output Resolution").with_page("Common"),
        "resolutionw" => Param::int(1280).with_label("W").with_range(1.0, 16384.0).with_page("Common"),
        "resolutionh" => Param::int(720).with_label("H").with_range(1.0, 16384.0).with_page("Common"),
        "filtertype" => Param::menu("linear", &["nearest", "linear", "mipmap"]).with_label("Viewer Smoothness").with_page("Common"),
        "fillmode" => Param::menu("nativeres", &[
            "useinput", "fill", "width", "height", "best", "outside", "nativeres",
        ]).with_label("Fill Viewer").with_page("Common"),
    }
}

pub fn movie_out_length_limit_frames(node: &Node, ctx: &EvalContext, fps: f64) -> Option<u64> {
    let on = val(node, ctx, "limitlength").as_bool();
    if !on {
        return None;
    }
    let length = val(node, ctx, "length").as_f64().max(0.0);
    let unit = val(node, ctx, "lengthunit").as_str();
    Some(match unit.as_str() {
        "seconds" => (length * fps.max(1e-6)).round() as u64,
        "frames" => length.round() as u64,
        _ => length.round() as u64,
    })
}

pub(crate) fn params_chop_to_top() -> IndexMap<String, Param> {
    params! {
        "layout" => Param::menu("mono", &["mono", "rgba"])
            .with_label("Channel Layout"),
    }
}

pub(crate) fn params_movie_in() -> IndexMap<String, Param> {
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

pub(crate) fn params_video_device_in() -> IndexMap<String, Param> {
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

pub(crate) fn params_render() -> IndexMap<String, Param> {
    with_res(params! {
        "geometry" => Param::str("").with_label("Geometry").as_path_ref(),
        "camera" => Param::str("").with_label("Camera").as_path_ref(),
        "light" => Param::str("").with_label("Light").as_path_ref(),
        "background" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Background"),
        "ambient" => Param::float(0.12).with_label("Ambient").with_range(0.0, 1.0),
        "wireframe" => Param::bool(false).with_label("Wireframe"),
        "cull" => Param::menu("back", &["back", "front", "none"]).with_label("Cull"),
        // A depth pass is the same draw with the shading skipped, so it is a
        // mode on this operator rather than a second one you have to keep in
        // sync with it — the camera, the geometry and the resolution all have
        // to match for the result to line up with the colour render, and the
        // surest way to guarantee that is for there to be one of each.
        "output" => Param::menu("color", &["color", "depth"]).with_label("Output"),
        "depthnear" => Param::float(0.1).with_label("Depth Near").with_range(0.0, 1000.0),
        "depthfar" => Param::float(20.0).with_label("Depth Far").with_range(0.01, 10000.0),
    })
}

// -------------------------------------------------------------- select TOP

pub(crate) fn params_select() -> IndexMap<String, Param> {
    params! {
        "top" => Param::str("").with_label("TOP"),
    }
}

// --------------------------------------------------------------- cache TOP

pub(crate) fn params_cache() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
    }
}

// ---------------------------------------------------------- resolution TOP

pub(crate) fn params_resolution() -> IndexMap<String, Param> {
    with_res(params! {
        "filter" => Param::menu("linear", &["linear", "nearest"]).with_label("Filter"),
    })
}

// ------------------------------------------------------------ displace TOP

pub(crate) fn params_displace() -> IndexMap<String, Param> {
    params! {
        "amount" => Param::float(0.1).with_label("Amount").with_range(-1.0, 1.0),
        "sourcex" => Param::menu("r", &["r", "g", "b", "a", "luminance"]).with_label("Source X"),
        "sourcey" => Param::menu("g", &["r", "g", "b", "a", "luminance"]).with_label("Source Y"),
        "offset" => Param::float(-0.5).with_label("Offset").with_range(-1.0, 1.0),
        "extend" => Param::menu("hold", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
    }
}

pub(crate) fn pack_displace(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "amount"),
            menu(n, c, "sourcex"),
            menu(n, c, "sourcey"),
            f(n, c, "offset"),
        ],
        [menu(n, c, "extend"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

// ---------------------------------------------------------------- GLSL TOP

/// The starting shader in a new GLSL TOP — a moving plasma, so the operator
/// does something the moment it is created.
const DEFAULT_WGSL: &str = "\
// WGSL fragment. Available: in.uv, U.time.x (seconds), U.res.xy,
// sample0(uv) / sample1(uv) for the two inputs, U.p0..U.p3 (Uniform 1..4).
let p = (in.uv - 0.5) * vec2<f32>(U.res.x / U.res.y, 1.0);
let d = length(p) * 8.0 - U.time.x * 2.0;
let v = 0.5 + 0.5 * sin(d + atan2(p.y, p.x) * 3.0);
return vec4<f32>(v * U.p0.rgb, 1.0);
";

pub(crate) fn params_glsl() -> IndexMap<String, Param> {
    with_res(params! {
        "language" => Param::menu("wgsl", &["wgsl", "glsl"]).with_label("Language"),
        "source" => Param::str(DEFAULT_WGSL).with_label("Source"),
        "uniform1" => Param::rgba([1.0, 0.6, 0.2, 1.0]).with_label("Uniform 1"),
        "uniform2" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Uniform 2"),
        "uniform3" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Uniform 3"),
        "uniform4" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Uniform 4"),
    })
}

/// The four generic Uniform parameters, for a hand-written shader.
pub(crate) fn pack_glsl(n: &Node, c: &EvalContext) -> PackedParams {
    let mut out = [[0.0f32; 4]; 4];
    out.copy_from_slice(&pack_glsl_wide(n, c)[..4]);
    out
}

pub(crate) fn pack_glsl_wide(n: &Node, c: &EvalContext) -> WideParams {
    let plain = || {
        widen([
            v4(n, c, "uniform1"),
            v4(n, c, "uniform2"),
            v4(n, c, "uniform3"),
            v4(n, c, "uniform4"),
        ])
    };

    // A GLSL TOP with custom parameters — the shape ISF import leaves behind,
    // and one an author can build by hand — packs those instead of the four
    // generic uniforms. `crate::isf::layout` decides where each one lands, and
    // the shader's `#define`s were written from that same function, so the two
    // sides agree by construction rather than by comment.
    let custom: Vec<(&String, &Param)> = n
        .params
        .iter()
        .filter(|(_, p)| p.custom && isf::width(&p.value) > 0)
        .collect();
    if custom.is_empty() {
        return plain();
    }
    let Some(slots) = isf::layout(custom.iter().map(|(_, p)| isf::width(&p.value))) else {
        // More parameters than uniform space. Import refuses this case; a
        // hand-built node can still reach it, and falling back is better than
        // writing values into the wrong slots.
        return plain();
    };

    let mut out = NO_PARAMS;
    for ((key, param), slot) in custom.iter().zip(slots) {
        let value = param.eval(c);
        match slot.width {
            4 => out[slot.vec] = value.as_vec4_f32(),
            2 => {
                let v = value.as_vec4_f32();
                out[slot.vec][slot.component] = v[0];
                out[slot.vec][slot.component + 1] = v[1];
            }
            _ => {
                out[slot.vec][slot.component] = if param.menu.is_some() {
                    menu(n, c, key)
                } else {
                    value.as_f32()
                };
            }
        }
    }
    out
}

// ---------------------------------------------------------------- dither TOP

pub(crate) fn params_dither() -> IndexMap<String, Param> {
    params! {
        "levels" => Param::int(4).with_label("Levels").with_range(2.0, 64.0),
        "pattern" => Param::menu("bayer4", &["bayer4", "bayer8", "noise", "none"])
            .with_label("Pattern"),
        "strength" => Param::float(1.0).with_label("Strength").with_range(0.0, 1.0),
        "scale" => Param::float(1.0).with_label("Pixel Size").with_range(1.0, 32.0),
        "monochrome" => Param::bool(false).with_label("Monochrome"),
    }
}

pub(crate) fn pack_dither(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "levels"),
            menu(n, c, "pattern"),
            f(n, c, "strength"),
            b(n, c, "monochrome"),
        ],
        [f(n, c, "scale"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

// --------------------------------------------------------------- voronoi TOP

pub(crate) fn params_voronoi() -> IndexMap<String, Param> {
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

pub(crate) fn pack_voronoi(n: &Node, c: &EvalContext) -> PackedParams {
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

// ------------------------------------------------------------------ toon TOP

pub(crate) fn params_toon() -> IndexMap<String, Param> {
    params! {
        "bands" => Param::int(4).with_label("Bands").with_range(2.0, 32.0),
        "edge" => Param::float(2.0).with_label("Ink Strength").with_range(0.0, 16.0),
        "edgewidth" => Param::float(1.0).with_label("Ink Width (px)").with_range(0.0, 8.0),
        "saturation" => Param::float(1.3).with_label("Saturation").with_range(0.0, 4.0),
        "inkcolor" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Ink Color"),
    }
}

pub(crate) fn pack_toon(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "bands"),
            f(n, c, "edge"),
            f(n, c, "edgewidth"),
            f(n, c, "saturation"),
        ],
        v4(n, c, "inkcolor"),
        [0.0; 4],
        [0.0; 4],
    ]
}

// ------------------------------------------------------------------ flow TOP

pub(crate) fn params_flow() -> IndexMap<String, Param> {
    params! {
        "amount" => Param::float(6.0).with_label("Amount (px)").with_range(0.0, 128.0),
        "scale" => Param::float(3.0).with_label("Field Scale").with_range(0.1, 32.0),
        "speed" => Param::float(0.3).with_label("Field Speed").with_range(0.0, 8.0),
        "usefield" => Param::bool(false).with_label("Steer From Input 2"),
        "fieldmix" => Param::float(1.0).with_label("Field Mix").with_range(0.0, 1.0),
    }
}

pub(crate) fn pack_flow(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "amount"),
            f(n, c, "scale"),
            f(n, c, "speed"),
            b(n, c, "usefield"),
        ],
        [f(n, c, "fieldmix"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

/// Whether a video node has been given nothing to open.
///
/// A blank `file` means no clip and nothing to do. A blank `device` means the
/// *default* camera — which is what that parameter's own label says, "blank =
/// default", and what `video::Source::camera` resolves against the platform's
/// first capture input.
///
/// The two were treated the same once, and the result was that a Video Device
/// In produced black for ever the moment you created one. No status on the
/// node, no ffmpeg started, nothing to find: the failure came *before* any
/// question of camera permissions, so every investigation went to the wrong
/// place. Its own function so the distinction can be tested without a camera,
/// a GPU or a cook.
pub fn nothing_to_open(is_camera: bool, source: &str) -> bool {
    !is_camera && source.trim().is_empty()
}

/// The shader source and language a GLSL TOP is currently set to.
pub fn shader_source(node: &Node, ctx: &EvalContext) -> (String, bool) {
    let source = val(node, ctx, "source").as_str();
    let is_glsl = val(node, ctx, "language").as_str() == "glsl";
    (source, is_glsl)
}

// ------------------------------------------------- the TouchDesigner filters
//
// Everything below is a single .wgsl file plus a `params_*`/`pack_*` pair, as
// the module header promises. They are grouped here rather than interleaved
// above because they arrived together, filling in the operators a
// TouchDesigner user reaches for and did not find.

pub(crate) fn params_monochrome() -> IndexMap<String, Param> {
    params! {
        "mode" => Param::menu(
            "luminance",
            &["luminance", "average", "maximum", "minimum", "red", "green", "blue", "alpha"],
        ).with_label("Mode"),
        "opacity" => Param::float(1.0).with_label("Opacity").with_range(0.0, 1.0),
    }
}

pub(crate) fn pack_monochrome(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "mode"), f(n, c, "opacity"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_channel_mix() -> IndexMap<String, Param> {
    params! {
        "red" => Param::new(Value::Vec4([1.0, 0.0, 0.0, 0.0])).with_label("Red From RGBA"),
        "green" => Param::new(Value::Vec4([0.0, 1.0, 0.0, 0.0])).with_label("Green From RGBA"),
        "blue" => Param::new(Value::Vec4([0.0, 0.0, 1.0, 0.0])).with_label("Blue From RGBA"),
        "alpha" => Param::new(Value::Vec4([0.0, 0.0, 0.0, 1.0])).with_label("Alpha From RGBA"),
    }
}

pub(crate) fn pack_channel_mix(n: &Node, c: &EvalContext) -> PackedParams {
    [
        v4(n, c, "red"),
        v4(n, c, "green"),
        v4(n, c, "blue"),
        v4(n, c, "alpha"),
    ]
}

const REORDER_SOURCES: &[&str] = &[
    "input1 r",
    "input1 g",
    "input1 b",
    "input1 a",
    "input2 r",
    "input2 g",
    "input2 b",
    "input2 a",
    "input1 luma",
    "input2 luma",
    "one",
    "zero",
];

pub(crate) fn params_reorder() -> IndexMap<String, Param> {
    params! {
        "red" => Param::menu("input1 r", REORDER_SOURCES).with_label("Red"),
        "green" => Param::menu("input1 g", REORDER_SOURCES).with_label("Green"),
        "blue" => Param::menu("input1 b", REORDER_SOURCES).with_label("Blue"),
        "alpha" => Param::menu("input1 a", REORDER_SOURCES).with_label("Alpha"),
    }
}

pub(crate) fn pack_reorder(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "red"),
            menu(n, c, "green"),
            menu(n, c, "blue"),
            menu(n, c, "alpha"),
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_matte() -> IndexMap<String, Param> {
    params! {
        "source" => Param::menu("alpha", &["alpha", "luminance", "red"]).with_label("Matte From"),
        "invert" => Param::bool(false).with_label("Invert"),
        "premultiply" => Param::bool(false).with_label("Premultiply"),
    }
}

pub(crate) fn pack_matte(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "source"),
            b(n, c, "invert"),
            b(n, c, "premultiply"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_rgb_key() -> IndexMap<String, Param> {
    params! {
        "color" => Param::rgba([0.0, 1.0, 0.0, 1.0]).with_label("Key Color"),
        "tolerance" => Param::float(0.2).with_label("Tolerance").with_range(0.0, 1.732),
        "softness" => Param::float(0.1).with_label("Softness").with_range(0.0, 1.0),
        "output" => Param::menu("keyed", &["keyed", "matte", "inverted matte"]).with_label("Output"),
    }
}

pub(crate) fn pack_rgb_key(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "tolerance"),
            f(n, c, "softness"),
            menu(n, c, "output"),
            0.0,
        ],
        v4(n, c, "color"),
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_luma_level() -> IndexMap<String, Param> {
    params! {
        "brightness" => Param::float(1.0).with_label("Brightness").with_range(0.0, 4.0),
        "contrast" => Param::float(1.0).with_label("Contrast").with_range(0.0, 4.0),
        "gamma" => Param::float(1.0).with_label("Gamma").with_range(0.1, 4.0),
        "opacity" => Param::float(1.0).with_label("Opacity").with_range(0.0, 1.0),
        "blacklevel" => Param::float(0.0).with_label("Black Level").with_range(0.0, 1.0),
        "whitelevel" => Param::float(1.0).with_label("White Level").with_range(0.0, 1.0),
    }
}

pub(crate) fn pack_luma_level(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "brightness"),
            f(n, c, "contrast"),
            f(n, c, "gamma"),
            f(n, c, "opacity"),
        ],
        [f(n, c, "blacklevel"), f(n, c, "whitelevel"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_function() -> IndexMap<String, Param> {
    params! {
        "function" => Param::menu(
            "none",
            &[
                "none", "log", "exp", "sqrt", "square", "reciprocal", "invert",
                "sin", "cos", "atan", "abs", "sign",
            ],
        ).with_label("Function"),
        "premultiply" => Param::float(1.0).with_label("Pre Multiply").with_range(-4.0, 4.0),
        "preadd" => Param::float(0.0).with_label("Pre Add").with_range(-4.0, 4.0),
        "postmultiply" => Param::float(1.0).with_label("Post Multiply").with_range(-4.0, 4.0),
        "postadd" => Param::float(0.0).with_label("Post Add").with_range(-4.0, 4.0),
        "alpha" => Param::bool(false).with_label("Include Alpha"),
    }
}

pub(crate) fn pack_function(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "function"),
            f(n, c, "premultiply"),
            f(n, c, "preadd"),
            f(n, c, "postmultiply"),
        ],
        [f(n, c, "postadd"), b(n, c, "alpha"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_limit() -> IndexMap<String, Param> {
    params! {
        "mode" => Param::menu("clamp", &["clamp", "quantize", "both"]).with_label("Mode"),
        "min" => Param::float(0.0).with_label("Minimum").with_range(-2.0, 2.0),
        "max" => Param::float(1.0).with_label("Maximum").with_range(-2.0, 2.0),
        "step" => Param::float(0.25).with_label("Quantize Step").with_range(0.001, 1.0),
        "alpha" => Param::bool(false).with_label("Include Alpha"),
    }
}

pub(crate) fn pack_limit(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "mode"),
            f(n, c, "min"),
            f(n, c, "max"),
            f(n, c, "step"),
        ],
        [b(n, c, "alpha"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_emboss() -> IndexMap<String, Param> {
    params! {
        "direction" => Param::float(45.0).with_label("Direction").with_range(-180.0, 180.0),
        "width" => Param::float(1.0).with_label("Width (px)").with_range(0.0, 32.0),
        "strength" => Param::float(4.0).with_label("Strength").with_range(0.0, 32.0),
        "mix" => Param::float(0.0).with_label("Mix With Source").with_range(0.0, 1.0),
    }
}

pub(crate) fn pack_emboss(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "direction").to_radians(),
            f(n, c, "width"),
            f(n, c, "strength"),
            f(n, c, "mix"),
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_slope() -> IndexMap<String, Param> {
    params! {
        "mode" => Param::menu("xy", &["xy", "magnitude", "direction"]).with_label("Output"),
        "strength" => Param::float(1.0).with_label("Strength").with_range(0.0, 32.0),
    }
}

pub(crate) fn pack_slope(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "mode"), f(n, c, "strength"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_normal_map() -> IndexMap<String, Param> {
    params! {
        "source" => Param::menu("luminance", &["luminance", "alpha", "red"]).with_label("Height From"),
        "strength" => Param::float(4.0).with_label("Strength").with_range(0.0, 64.0),
        "flipgreen" => Param::bool(false).with_label("Flip Green"),
    }
}

pub(crate) fn pack_normal_map(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "source"),
            f(n, c, "strength"),
            b(n, c, "flipgreen"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_antialias() -> IndexMap<String, Param> {
    params! {
        "strength" => Param::float(1.0).with_label("Strength").with_range(0.0, 2.0),
    }
}

pub(crate) fn pack_antialias(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [f(n, c, "strength"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_convolve() -> IndexMap<String, Param> {
    params! {
        "row0" => Param::xyz([0.0, 0.0, 0.0]).with_label("Kernel Row 1"),
        "row1" => Param::xyz([0.0, 1.0, 0.0]).with_label("Kernel Row 2"),
        "row2" => Param::xyz([0.0, 0.0, 0.0]).with_label("Kernel Row 3"),
        "spread" => Param::float(1.0).with_label("Spread (px)").with_range(0.0, 32.0),
        "normalize" => Param::bool(true).with_label("Normalize"),
        "scale" => Param::float(1.0).with_label("Scale").with_range(-4.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-1.0, 1.0),
    }
}

pub(crate) fn pack_convolve(n: &Node, c: &EvalContext) -> PackedParams {
    let r0 = val(n, c, "row0").as_vec4_f32();
    let r1 = val(n, c, "row1").as_vec4_f32();
    let r2 = val(n, c, "row2").as_vec4_f32();
    [
        [r0[0], r0[1], r0[2], f(n, c, "spread")],
        [r1[0], r1[1], r1[2], f(n, c, "scale")],
        [r2[0], r2[1], r2[2], f(n, c, "offset")],
        [b(n, c, "normalize"), 0.0, 0.0, 0.0],
    ]
}

pub(crate) fn params_corner_pin() -> IndexMap<String, Param> {
    params! {
        "bottomleft" => Param::new(Value::Vec2([0.0, 0.0])).with_label("Bottom Left"),
        "bottomright" => Param::new(Value::Vec2([1.0, 0.0])).with_label("Bottom Right"),
        "topleft" => Param::new(Value::Vec2([0.0, 1.0])).with_label("Top Left"),
        "topright" => Param::new(Value::Vec2([1.0, 1.0])).with_label("Top Right"),
        "extend" => Param::menu("zero", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
    }
}

pub(crate) fn pack_corner_pin(n: &Node, c: &EvalContext) -> PackedParams {
    let bl = val(n, c, "bottomleft").as_vec4_f32();
    let br = val(n, c, "bottomright").as_vec4_f32();
    let tl = val(n, c, "topleft").as_vec4_f32();
    let tr = val(n, c, "topright").as_vec4_f32();
    [
        [bl[0], bl[1], br[0], br[1]],
        [tl[0], tl[1], tr[0], tr[1]],
        [menu(n, c, "extend"), 0.0, 0.0, 0.0],
        [0.0; 4],
    ]
}

pub(crate) fn params_crop() -> IndexMap<String, Param> {
    with_res(params! {
        "left" => Param::float(0.0).with_label("Left").with_range(0.0, 1.0),
        "right" => Param::float(1.0).with_label("Right").with_range(0.0, 1.0),
        "bottom" => Param::float(0.0).with_label("Bottom").with_range(0.0, 1.0),
        "top" => Param::float(1.0).with_label("Top").with_range(0.0, 1.0),
    })
}

pub(crate) fn pack_crop(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "left"),
            f(n, c, "right"),
            f(n, c, "bottom"),
            f(n, c, "top"),
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_fit() -> IndexMap<String, Param> {
    with_res(params! {
        "mode" => Param::menu("fit", &["fill", "fit", "stretch", "native"]).with_label("Fit"),
        "background" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Background"),
    })
}

pub(crate) fn pack_fit(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "mode"), 0.0, 0.0, 0.0],
        v4(n, c, "background"),
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_lens_distort() -> IndexMap<String, Param> {
    params! {
        "k1" => Param::float(0.0).with_label("Distort (- bulges, + corrects)").with_range(-1.0, 1.0),
        "k2" => Param::float(0.0).with_label("Distort Squared").with_range(-1.0, 1.0),
        "scale" => Param::float(1.0).with_label("Scale").with_range(0.1, 4.0),
        "center" => Param::new(Value::Vec2([0.5, 0.5])).with_label("Center"),
        "extend" => Param::menu("zero", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
    }
}

pub(crate) fn pack_lens_distort(n: &Node, c: &EvalContext) -> PackedParams {
    let ctr = val(n, c, "center").as_vec4_f32();
    [
        [
            f(n, c, "k1"),
            f(n, c, "k2"),
            f(n, c, "scale"),
            menu(n, c, "extend"),
        ],
        [ctr[0], ctr[1], 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_remap() -> IndexMap<String, Param> {
    params! {
        "amount" => Param::float(1.0).with_label("Amount").with_range(0.0, 1.0),
        "extend" => Param::menu("zero", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
        "flipv" => Param::bool(false).with_label("Flip V"),
    }
}

pub(crate) fn pack_remap(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "amount"),
            menu(n, c, "extend"),
            b(n, c, "flipv"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_tile() -> IndexMap<String, Param> {
    params! {
        "repeat" => Param::new(Value::Vec2([2.0, 2.0])).with_label("Repeat"),
        "offset" => Param::new(Value::Vec2([0.0, 0.0])).with_label("Offset"),
        "mirror" => Param::bool(false).with_label("Mirror"),
        "rowshift" => Param::float(0.0).with_label("Row Shift").with_range(0.0, 1.0),
        "columnshift" => Param::float(0.0).with_label("Column Shift").with_range(0.0, 1.0),
    }
}

pub(crate) fn pack_tile(n: &Node, c: &EvalContext) -> PackedParams {
    let r = val(n, c, "repeat").as_vec4_f32();
    let o = val(n, c, "offset").as_vec4_f32();
    [
        [r[0], r[1], o[0], o[1]],
        [
            b(n, c, "mirror"),
            f(n, c, "rowshift"),
            f(n, c, "columnshift"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_luma_blur() -> IndexMap<String, Param> {
    params! {
        "white" => Param::float(16.0).with_label("White Radius (px)").with_range(0.0, 128.0),
        "black" => Param::float(0.0).with_label("Black Radius (px)").with_range(0.0, 128.0),
        "source" => Param::menu("luminance", &["luminance", "alpha", "red"]).with_label("Radius From"),
    }
}

pub(crate) fn pack_luma_blur(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "white"),
            f(n, c, "black"),
            menu(n, c, "source"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub(crate) fn params_optical_flow() -> IndexMap<String, Param> {
    params! {
        "gain" => Param::float(1.0).with_label("Vector Gain").with_range(0.0, 20.0),
        "regularization" => Param::float(0.0001).with_label("Noise Regularization").with_range(0.000001, 0.1),
        "maxmotion" => Param::float(4.0).with_label("Maximum Motion (px/frame)").with_range(0.1, 16.0),
    }
}

pub(crate) fn pack_optical_flow(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "gain"),
            f(n, c, "regularization"),
            f(n, c, "maxmotion"),
            0.0,
        ],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}
