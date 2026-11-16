use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use crate::isf;

use super::helpers::*;
use super::{NO_PARAMS, PackedParams, Sizing, TopCook, TopSpec, WideParams, widen};

pub const TYPE_NAME: &str = "glslTOP";
pub const GLSL: &str = TYPE_NAME;

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

fn params() -> IndexMap<String, Param> {
    let mut m = with_res(params! {
        "language" => Param::menu("wgsl", &["wgsl", "glsl"]).with_label("Language"),
        "source" => Param::str(DEFAULT_WGSL).with_label("Source"),
        "pixeldat" => Param::str("").with_label("Pixel Shader").with_page("GLSL").as_path_ref(),
        "computedat" => Param::str("").with_label("Compute Shader").with_page("GLSL").as_path_ref(),
        "glslversion" => Param::menu("glsl450", &["glsl120", "glsl330", "glsl450", "glsl460"])
            .with_label("GLSL Version")
            .with_page("GLSL"),
        "uniform1" => Param::rgba([1.0, 0.6, 0.2, 1.0]).with_label("Uniform 1"),
        "uniform2" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Uniform 2"),
        "uniform3" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Uniform 3"),
        "uniform4" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Uniform 4"),
    });
    for i in 0..=3 {
        m.insert(
            format!("vec{i}name"),
            Param::str("")
                .with_label(&format!("Vector {i} Name"))
                .with_page("Vectors"),
        );
        for (suffix, axis) in [("x", "X"), ("y", "Y"), ("z", "Z"), ("w", "W")] {
            m.insert(
                format!("vec{i}value{suffix}"),
                Param::float(0.0)
                    .with_label(&format!("Vector {i} {axis}"))
                    .with_page("Vectors"),
            );
        }
    }
    for i in 0..=7 {
        m.insert(
            format!("const{i}name"),
            Param::str("")
                .with_label(&format!("Constant {i} Name"))
                .with_page("Constants"),
        );
        m.insert(
            format!("const{i}value"),
            Param::float(0.0)
                .with_label(&format!("Constant {i} Value"))
                .with_page("Constants"),
        );
    }
    m
}

/// The four generic Uniform parameters, for a hand-written shader.
fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let mut out = [[0.0f32; 4]; 4];
    out.copy_from_slice(&pack_wide(n, c)[..4]);
    out
}

pub fn pack_wide(n: &Node, c: &EvalContext) -> WideParams {
    let plain = || {
        let mut out = widen([
            v4(n, c, "uniform1"),
            v4(n, c, "uniform2"),
            v4(n, c, "uniform3"),
            v4(n, c, "uniform4"),
        ]);
        for (i, slot) in out.iter_mut().enumerate() {
            if let Some(value) = named_vec(n, c, i) {
                *slot = value;
            }
        }
        out
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
                    // The stored value is what the shader sees. A non-numeric
                    // choice (a name) falls back to its place in the menu.
                    value.as_str().parse().unwrap_or_else(|_| menu(n, c, key))
                } else {
                    value.as_f32()
                };
            }
        }
    }
    out
}

/// The shader source and language a GLSL TOP is currently set to.
pub fn shader_source(node: &Node, ctx: &EvalContext) -> (String, bool) {
    let mut source = val(node, ctx, "source").as_str();
    if let Some(from_dat) = pixel_shader_text(node, ctx) {
        if !from_dat.trim().is_empty() {
            source = from_dat;
        }
    }
    let looks_glsl = source.contains("void main")
        || source.contains("fragColor")
        || source.contains("sTD2DInputs")
        || source.contains("#version");
    let is_glsl = val(node, ctx, "language").as_str() == "glsl" || looks_glsl;
    if is_glsl {
        let bindings = named_vec_bindings(node, ctx);
        let refs: Vec<(&str, &str)> = bindings
            .iter()
            .map(|(name, slot)| (name.as_str(), slot.as_str()))
            .collect();
        source = crate::shader::adopt_td_uniforms(&source, &refs);
    }
    (source, is_glsl)
}

fn pixel_shader_text(node: &Node, ctx: &EvalContext) -> Option<String> {
    let reference = val(node, ctx, "pixeldat").as_str();
    let reference = reference.trim();
    if reference.is_empty() {
        return None;
    }
    let net = ctx.channels?;
    let from = ctx.path.unwrap_or("");
    let path = net.resolve_operator_path(from, reference)?;
    net.dat_text(&path)
}

fn named_vec(node: &Node, ctx: &EvalContext, index: usize) -> Option<[f32; 4]> {
    let name = val(node, ctx, &format!("vec{index}name")).as_str();
    let name = name.trim().trim_matches('"');
    if !is_glsl_ident(name) {
        return None;
    }
    Some([
        f(node, ctx, &format!("vec{index}valuex")),
        f(node, ctx, &format!("vec{index}valuey")),
        f(node, ctx, &format!("vec{index}valuez")),
        f(node, ctx, &format!("vec{index}valuew")),
    ])
}

fn named_vec_bindings(node: &Node, ctx: &EvalContext) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for i in 0..4 {
        let name = val(node, ctx, &format!("vec{i}name")).as_str();
        let name = name.trim().trim_matches('"').to_string();
        if !is_glsl_ident(&name) {
            continue;
        }
        out.push((name, format!("U.p{i}")));
    }
    out
}

fn is_glsl_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "GLSL",
            family: Family::Top,
            inputs: &["in0", "in1"],
            summary: "Your own shader, compiled live. WGSL or Shadertoy-style GLSL.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: "",
        sizing: Sizing::Input0OrParams,
        two_pass: false,
        dynamic_shader: true,
        pack,
        cook: TopCook::Shader,
    }
}
