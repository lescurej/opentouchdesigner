use otd_core::indexmap::IndexMap;
use otd_core::{EvalContext, Node, Param, Value};

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
    param
        .menu
        .as_ref()
        .and_then(|items| items.iter().position(|i| *i == chosen))
        .unwrap_or(0) as f32
}

/// Output resolution of a generator.
pub fn generator_size(node: &Node, ctx: &EvalContext) -> (u32, u32) {
    let w = val(node, ctx, "resw").as_i64().clamp(1, 16384) as u32;
    let h = val(node, ctx, "resh").as_i64().clamp(1, 16384) as u32;
    (w, h)
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

pub(crate) fn with_res(mut m: IndexMap<String, Param>) -> IndexMap<String, Param> {
    m.insert(
        "resw".into(),
        Param::int(1280)
            .with_label("Resolution W")
            .with_range(1.0, 4096.0),
    );
    m.insert(
        "resh".into(),
        Param::int(720)
            .with_label("Resolution H")
            .with_range(1.0, 4096.0),
    );
    m
}

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}
