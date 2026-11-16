use otd_core::indexmap::IndexMap;
use otd_core::Param;
use otd_core::Crossing;
use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "top" => Param::str("").with_label("TOP").as_path_ref(),
        "active" => Param::bool(true).with_label("Active"),
        "layout" => Param::menu("rows", &["rows", "columns", "average", "image"])
            .with_label("Read"),
        "index" => Param::int(0)
            .with_label("Row / Column")
            .with_range(0.0, 4096.0),
    }
}

/// Pixels as channels: `r`, `g`, `b`, `a`, one sample per pixel read.
///
/// Two things about this operator are worth knowing before you patch it, and
/// both are true of TouchDesigner's as well:
///
///  * It reads **last frame**. A readback has to be submitted to the GPU, and
///    this frame's work is still being recorded into an encoder that has not
///    been submitted yet. Waiting for it would mean stalling the frame on the
///    render it is in the middle of building.
///  * It costs a full pipeline sync, and the cost is the *source's*
///    resolution. Put a Resolution TOP in front of it. Reading 1920×1080 back
///    every frame to get four numbers is the classic way to turn a 60 fps
///    patch into a 12 fps one.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    if !c.b("active") {
        return ChopData::empty();
    }
    let Some(Crossing::Pixels {
        width,
        height,
        rgba,
    }) = c.foreign(0)
    else {
        return ChopData::empty();
    };
    let (w, h) = (*width as usize, *height as usize);
    if w == 0 || h == 0 {
        return ChopData::empty();
    }
    let at = |x: usize, y: usize, comp: usize| rgba[(y * w + x) * 4 + comp];

    let mut samples = vec![Vec::new(); 4];
    match c.menu("layout") {
        // One row of pixels, left to right.
        0 => {
            let y = (c.i("index").max(0) as usize).min(h - 1);
            for (comp, out) in samples.iter_mut().enumerate() {
                *out = (0..w).map(|x| at(x, y, comp)).collect();
            }
        }
        // One column, top to bottom.
        1 => {
            let x = (c.i("index").max(0) as usize).min(w - 1);
            for (comp, out) in samples.iter_mut().enumerate() {
                *out = (0..h).map(|y| at(x, y, comp)).collect();
            }
        }
        // Every pixel, top row first, plus where it is: `u` across and `v`
        // up, centred on zero with `u` spanning the aspect ratio. Named so a
        // Geometry COMP can instance straight off it — tx=u, ty=v, tz=g is a
        // luma-displaced point cloud of the picture with no other operator.
        3 => {
            // ponytail: 16384 pixels (128x128) caps the per-frame CPU copy;
            // a GPU instancing path is the upgrade.
            if w * h > 1 << 14 {
                return ChopData::empty();
            }
            for (comp, out) in samples.iter_mut().enumerate() {
                *out = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| at(x, y, comp)).collect();
            }
            let aspect = w as f32 / h as f32;
            let (u, v): (Vec<f32>, Vec<f32>) = (0..h)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .map(|(x, y)| (
                    ((x as f32 + 0.5) / w as f32 - 0.5) * aspect,
                    0.5 - (y as f32 + 0.5) / h as f32,
                ))
                .unzip();
            samples.push(u);
            samples.push(v);
        }
        // The whole image as one number per component.
        _ => {
            let n = (w * h) as f32;
            for (comp, out) in samples.iter_mut().enumerate() {
                let sum: f32 = (0..h)
                    .flat_map(|y| (0..w).map(move |x| (x, y)))
                    .map(|(x, y)| at(x, y, comp))
                    .sum();
                *out = vec![sum / n];
            }
        }
    }

    let names = ["r", "g", "b", "a", "u", "v"];
    let channels = samples
        .into_iter()
        .enumerate()
        .map(|(i, s)| Channel::new(names[i], s))
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}


pub const TYPE_NAME: &str = "toptochopCHOP";

pub fn spec() -> ChopSpec {
    let mut s = super::converter_spec(
        TYPE_NAME,
        "TOP to CHOP",
        "Pixels as channels r g b a. `image` reads every pixel (top row first, at most \
         128x128) and adds u/v positions, centred, v up — instance a Geometry COMP with \
         tx=u ty=v tz=g for a camera point cloud. Reads last frame; costs a GPU sync, \
         so put a Resolution TOP in front.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
