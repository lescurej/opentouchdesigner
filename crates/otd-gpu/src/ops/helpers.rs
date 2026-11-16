use otd_core::indexmap::IndexMap;
use otd_core::{EvalContext, Node, Param, Value};

use super::PackedParams;

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}

// ------------------------------------------------------------ param helpers

pub(crate) fn val(node: &Node, ctx: &EvalContext, key: &str) -> Value {
    node.param(key)
        .map(|p| p.eval(ctx))
        .unwrap_or(Value::Float(0.0))
}

pub(crate) fn f(node: &Node, ctx: &EvalContext, key: &str) -> f32 {
    val(node, ctx, key).as_f32()
}

pub(crate) fn b(node: &Node, ctx: &EvalContext, key: &str) -> f32 {
    if val(node, ctx, key).as_bool() {
        1.0
    } else {
        0.0
    }
}

pub(crate) fn v4(node: &Node, ctx: &EvalContext, key: &str) -> [f32; 4] {
    val(node, ctx, key).as_vec4_f32()
}

/// Menu parameters store their label; the shader wants the index.
pub(crate) fn menu(node: &Node, ctx: &EvalContext, key: &str) -> f32 {
    let Some(param) = node.param(key) else {
        return 0.0;
    };
    let chosen = param.eval(ctx).as_str();
    param.menu_index(&chosen) as f32
}

/// Output resolution of a generator.
pub fn generator_size(node: &Node, ctx: &EvalContext) -> (u32, u32) {
    if node.param("outputresolution").is_some() {
        let mode = val(node, ctx, "outputresolution").as_str();
        if matches!(mode.as_str(), "custom" | "fit" | "limit") {
            let w = val(node, ctx, "resolutionw").as_i64().clamp(1, 16384) as u32;
            let h = val(node, ctx, "resolutionh").as_i64().clamp(1, 16384) as u32;
            return (w, h);
        }
    }
    let w = val(node, ctx, "resw").as_i64().clamp(1, 16384) as u32;
    let h = val(node, ctx, "resh").as_i64().clamp(1, 16384) as u32;
    (w, h)
}

pub(crate) fn params_common_page() -> IndexMap<String, Param> {
    params! {
        "outputresolution" => Param::menu("useinput", &[
            "useinput", "eighth", "quarter", "half", "2x", "4x", "8x", "fit", "limit", "custom",
        ])
            .with_label("Output Resolution")
            .with_page("Common"),
        "resolutionw" => Param::int(1280)
            .with_label("W")
            .with_range(1.0, 16384.0)
            .with_page("Common"),
        "resolutionh" => Param::int(720)
            .with_label("H")
            .with_range(1.0, 16384.0)
            .with_page("Common"),
        "resmult" => Param::bool(true)
            .with_label("Use Global Res Multiplier")
            .with_page("Common"),
        "outputaspect" => Param::menu("useinput", &["useinput", "resolution", "custom"])
            .with_label("Output Aspect")
            .with_page("Common"),
        "aspect1" => Param::float(1.0).with_label("Aspect1").with_page("Common"),
        "aspect2" => Param::float(1.0).with_label("Aspect2").with_page("Common"),
        "armenu" => Param::menu("16:9", &["16:9", "4:3", "1:1", "2.39:1"])
            .with_label("Aspect Menu")
            .with_page("Common"),
        "filtertype" => Param::menu("linear", &["nearest", "linear", "mipmap"])
            .with_label("Viewer Smoothness")
            .with_page("Common"),
        "fillmode" => Param::menu("nativeres", &[
            "useinput", "fill", "width", "height", "best", "outside", "nativeres",
        ])
            .with_label("Fill Viewer")
            .with_page("Common"),
        "npasses" => Param::int(1)
            .with_label("Passes")
            .with_range(1.0, 32.0)
            .with_page("Common"),
        "chanmask" => Param::str("rgba").with_label("Channel Mask").with_page("Common"),
        "format" => Param::menu("useinput", &[
            "useinput", "rgba8fixed", "srgba8fixed", "rgba16float", "rgba32float",
            "rgb10a2fixed", "rgba16fixed", "rgba11float", "rgb16float",
        ])
            .with_label("Pixel Format")
            .with_page("Common"),
    }
}

pub(crate) fn merge_common(mut base: IndexMap<String, Param>) -> IndexMap<String, Param> {
    for (k, v) in params_common_page() {
        base.entry(k).or_insert(v);
    }
    base
}

pub(crate) fn params_dual_input_sizing() -> IndexMap<String, Param> {
    merge_common(params! {
        "size" => Param::menu("input1", &["input1", "input2"])
            .with_label("Fixed Layer")
            .with_page("Transform"),
    })
}

pub(crate) fn append_params(
    base: IndexMap<String, Param>,
    extra: IndexMap<String, Param>,
) -> IndexMap<String, Param> {
    let mut out = base;
    for (k, v) in extra {
        out.insert(k, v);
    }
    out
}

fn resolution_input_index(node: &Node, ctx: &EvalContext) -> usize {
    match val(node, ctx, "size").as_str().as_str() {
        "input2" => 1,
        _ => 0,
    }
}

pub fn filter_output_dimensions<F>(
    node: &Node,
    ctx: &EvalContext,
    mut input_size: F,
) -> Option<(u32, u32)>
where
    F: FnMut(usize) -> Option<(u32, u32)>,
{
    let has_output_res = node.param("outputresolution").is_some();
    let has_fixed_layer = node.param("size").is_some();
    if !has_output_res && !has_fixed_layer {
        return input_size(0)
            .or_else(|| input_size(1))
            .map(|wh| apply_preshrink(node, ctx, wh));
    }

    let mut pick_input = || {
        let idx = if has_fixed_layer {
            resolution_input_index(node, ctx)
        } else {
            0
        };
        input_size(idx).or_else(|| input_size(1 - idx))
    };

    let mode = if has_output_res {
        val(node, ctx, "outputresolution").as_str()
    } else {
        "useinput".into()
    };

    let size = match mode.as_str() {
        "useinput" => pick_input(),
        "custom" | "fit" | "limit" => {
            let w = val(node, ctx, "resolutionw").as_i64().clamp(1, 16384) as u32;
            let h = val(node, ctx, "resolutionh").as_i64().clamp(1, 16384) as u32;
            Some((w, h))
        }
        _ => {
            let base = pick_input().unwrap_or((1280, 720));
            Some(movie_output_dimensions(node, ctx, base))
        }
    }?;
    Some(apply_preshrink(node, ctx, size))
}

fn apply_preshrink(node: &Node, ctx: &EvalContext, size: (u32, u32)) -> (u32, u32) {
    let Some(param) = node.param("preshrink") else {
        return size;
    };
    let n = param.eval(ctx).as_i64().max(1) as u32;
    ((size.0 / n).max(1), (size.1 / n).max(1))
}

pub fn movie_output_dimensions(node: &Node, ctx: &EvalContext, input: (u32, u32)) -> (u32, u32) {
    let (iw, ih) = (input.0.max(1), input.1.max(1));
    let scale = |n: u32| (iw / n).max(1);
    let scale_h = |n: u32| (ih / n).max(1);
    match val(node, ctx, "outputresolution").as_str().as_str() {
        "useinput" => (iw, ih),
        "eighth" => (scale(8), scale_h(8)),
        "quarter" => (scale(4), scale_h(4)),
        "half" => (scale(2), scale_h(2)),
        "2x" => (iw * 2, ih * 2),
        "4x" => (iw * 4, ih * 4),
        "8x" => (iw * 8, ih * 8),
        "custom" | "limit" | "fit" => {
            let w = val(node, ctx, "resolutionw").as_i64().clamp(1, 16384) as u32;
            let h = val(node, ctx, "resolutionh").as_i64().clamp(1, 16384) as u32;
            (w, h)
        }
        _ => (iw, ih),
    }
}

pub fn movie_fit_mode(node: &Node, ctx: &EvalContext) -> f32 {
    match val(node, ctx, "fillmode").as_str().as_str() {
        "fill" => 0.0,
        "best" | "outside" | "fit" | "useinput" => 1.0,
        "width" | "height" => 1.0,
        "stretch" => 2.0,
        "nativeres" => 3.0,
        _ => 3.0,
    }
}

pub fn movie_sampler_nearest(node: &Node, ctx: &EvalContext) -> bool {
    val(node, ctx, "filtertype").as_str().as_str() == "nearest"
}

pub(crate) fn params_none() -> IndexMap<String, Param> {
    merge_common(params! {})
}

pub(crate) fn pack_none(_n: &Node, _c: &EvalContext) -> PackedParams {
    [[0.0; 4]; 4]
}

pub(crate) fn with_res(m: IndexMap<String, Param>) -> IndexMap<String, Param> {
    let mut m = merge_common(m);
    m.insert(
        "resw".into(),
        Param::int(1280)
            .with_label("Resolution W")
            .with_range(1.0, 4096.0)
            .with_page("Common"),
    );
    m.insert(
        "resh".into(),
        Param::int(720)
            .with_label("Resolution H")
            .with_range(1.0, 4096.0)
            .with_page("Common"),
    );
    m
}
