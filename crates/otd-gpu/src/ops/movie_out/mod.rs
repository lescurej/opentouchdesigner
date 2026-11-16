use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "moviefileoutTOP";
pub const MOVIE_OUT: &str = TYPE_NAME;

pub fn length_limit_frames(node: &Node, ctx: &EvalContext, fps: f64) -> Option<u64> {
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

fn params() -> IndexMap<String, Param> {
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


fn pack(_n: &Node, _c: &EvalContext) -> PackedParams {
    [[0.0; 4]; 4]
}


pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Movie File Out",
            family: Family::Top,
            inputs: &["in"],
            summary: "Records its input to a movie file. Passes the picture through.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::MovieOut,
    }
}
