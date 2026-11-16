use super::hoist::hoist_const_arrays;

use crate::ops::COMMON_WGSL;

/// The prelude on its own, used as the vertex stage when the fragment stage
/// was written in GLSL and therefore lives in a separate module.
pub const VERTEX_ONLY_WGSL: &str = COMMON_WGSL;

/// The fullscreen triangle again, in GLSL.
///
/// A GLSL fragment stage cannot be paired with the WGSL vertex stage: naga's
/// GLSL front-end emits interpolation sampling `None` for a plain `in`, WGSL
/// emits `Center`, and wgpu rejects the mismatch. Compiling both stages
/// through the same front-end sidesteps it.
pub const VERTEX_GLSL: &str = r#"#version 450 core
layout(location = 0) out vec2 otd_uv;
void main() {
    float x = float((gl_VertexIndex << 1) & 2);
    float y = float(gl_VertexIndex & 2);
    otd_uv = vec2(x, y);
    gl_Position = vec4(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}
"#;

/// Wrap a WGSL fragment body, unless it declares its own entry point.
pub fn wrap_wgsl(source: &str) -> String {
    if source.contains("@fragment") {
        format!("{COMMON_WGSL}\n{source}")
    } else {
        format!(
            "{COMMON_WGSL}\n@fragment\nfn fs_main(in: VOut) -> @location(0) vec4<f32> {{\n{source}\n}}\n"
        )
    }
}

const GLSL_PREAMBLE: &str = r#"#version 450 core
layout(location = 0) in vec2 otd_uv;
layout(location = 0) out vec4 otd_frag;
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 res;
    vec4 time;
    vec4 p0;
    vec4 p1;
    vec4 p2;
    vec4 p3;
    vec4 p4;
    vec4 p5;
    vec4 p6;
    vec4 p7;
    vec4 p8;
    vec4 p9;
    vec4 p10;
    vec4 p11;
} U;
#define iResolution vec3(U.res.xy, 1.0)
#define iTime U.time.x
#define iTimeDelta (1.0 / max(U.time.w, 1.0))
#define iFrame int(U.time.z)
#define iMouse vec4(0.0)
#define iDate vec4(0.0)
"#;

/// Texture inputs, declared only when a shader actually samples them.
///
/// naga's GLSL front-end maps a combined `sampler2D` onto our separate
/// texture and sampler bindings, which is what lets an imported ISF shader
/// call `texture(img, uv)` unchanged.
const GLSL_SAMPLERS: &str = r#"layout(set = 0, binding = 1) uniform sampler otd_samp;
layout(set = 0, binding = 2) uniform texture2D otd_tex0;
layout(set = 0, binding = 3) uniform texture2D otd_tex1;
#define otd_image0 sampler2D(otd_tex0, otd_samp)
#define otd_image1 sampler2D(otd_tex1, otd_samp)
#define iChannel0 otd_image0
#define iChannel1 otd_image1
"#;

/// Shadertoy samples its channels bottom-up, like the `fragCoord` it hands
/// out. Our textures are top-down.
///
/// So a shader doing the most ordinary thing there is —
/// `texture(iChannel0, fragCoord / iResolution.xy)` — got the picture upside
/// down, silently, in a way that reads as a deliberate effect until you put a
/// camera on it and watch yourself hang from the ceiling. Every generated
/// shader that read its input had it, and so did two of the camera recipes.
///
/// Flipping `fragCoord` instead would fix the passthrough and mirror every
/// generative shader copied off shadertoy.com, so the flip goes where the
/// mismatch actually is: the lookup, and only for our own channels.
///
/// Rewritten at the call site rather than wrapped in a helper, because naga's
/// GLSL front end will not take a `sampler2D` as a function parameter — and
/// not as `#define texture(...)` either, because its preprocessor leaves the
/// builtin alone, so that one compiled, did nothing, and left the picture
/// upside down, which is worse than not trying.
fn is_td_pixel_shader(source: &str) -> bool {
    source.contains("sTD2DInputs")
        || source.contains("uTD2DInfos")
        || source.contains("uTDOutputInfo")
        || source.contains("TDOutputSwizzle")
}

const TD_PREAMBLE: &str = r#"#version 450 core
layout(location = 0) in vec2 otd_uv;
layout(set = 0, binding = 0) uniform Uniforms {
    vec4 res;
    vec4 time;
    vec4 p0;
    vec4 p1;
    vec4 p2;
    vec4 p3;
    vec4 p4;
    vec4 p5;
    vec4 p6;
    vec4 p7;
    vec4 p8;
    vec4 p9;
    vec4 p10;
    vec4 p11;
} U;
layout(set = 0, binding = 1) uniform sampler otd_samp;
layout(set = 0, binding = 2) uniform texture2D otd_tex0;
layout(set = 0, binding = 3) uniform texture2D otd_tex1;
#define vUV vec4(otd_uv.x, 1.0 - otd_uv.y, 0.0, 1.0)
vec4 tdFragCoord() { return vec4(gl_FragCoord.x, U.res.y - gl_FragCoord.y, gl_FragCoord.z, gl_FragCoord.w); }
vec4 TDOutputSwizzle(vec4 color) { return color; }
"#;

fn const_table_glsl(len: usize) -> String {
    if len == 0 {
        return String::new();
    }
    "layout(set = 0, binding = 4) uniform texture2D otd_table;\nfloat otdK(int i) { return texelFetch(otd_table, ivec2(i, 0), 0).r; }\n".to_string()
}

fn wrap_td_glsl(source: &str, table_len: usize) -> String {
    let body = source
        .replace("sTD2DInputs[0]", "sampler2D(otd_tex0, otd_samp)")
        .replace("sTD2DInputs[1]", "sampler2D(otd_tex1, otd_samp)")
        .replace(
            "uTD2DInfos[0].res",
            "vec4(U.res.z, U.res.w, U.res.x, U.res.y)",
        )
        .replace(
            "uTDOutputInfo.res",
            "vec4(U.res.z, U.res.w, U.res.x, U.res.y)",
        )
        .replace("gl_FragCoord", "tdFragCoord()");
    let output = if body.contains("out vec4") || body.contains("gl_FragColor") {
        ""
    } else {
        "layout(location = 0) out vec4 fragColor;\n"
    };
    let table = const_table_glsl(table_len);
    format!("{TD_PREAMBLE}{table}{output}{body}")
}

pub(crate) fn flip_channel_lookups(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let call: Vec<char> = "texture(".chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < chars.len() {
        // `texture(` cannot be the tail of `textureLod(` or `textureGrad(`,
        // but it can be the tail of an identifier somebody declared, so the
        // character before it has to be a non-identifier one.
        let starts_here = chars[i..].starts_with(&call)
            && (i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_'));
        let rewritten = starts_here
            .then(|| matching_paren(&chars, i + call.len() - 1))
            .flatten()
            .and_then(|close| {
                let inner: String = chars[i + call.len()..close].iter().collect();
                let args = split_args(&inner);
                // Two arguments, and the sampler is one of ours. Anything
                // else is left exactly as it was written.
                let ours = args.first().is_some_and(|a| {
                    let a = a.trim();
                    a.starts_with("iChannel") || a.starts_with("otd_image")
                });
                (args.len() == 2 && ours).then(|| {
                    (
                        format!(
                            "texture({}, vec2(({1}).x, 1.0 - ({1}).y))",
                            args[0].trim(),
                            args[1].trim()
                        ),
                        close,
                    )
                })
            });
        match rewritten {
            Some((text, close)) => {
                out.push_str(&text);
                i = close + 1;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// The index of the `)` closing the `(` at `open`.
fn matching_paren(chars: &[char], open: usize) -> Option<usize> {
    if chars.get(open) != Some(&'(') {
        return None;
    }
    let mut depth = 0i32;
    for (at, c) in chars.iter().enumerate().skip(open) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split an argument list on the commas that are not inside brackets.
fn split_args(inner: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                args.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    if !current.trim().is_empty() || !args.is_empty() {
        args.push(current);
    }
    args
}
/// Shadertoy's fragCoord has its origin bottom-left; ours is top-left.
const GLSL_MAIN: &str = r#"
void main() {
    vec2 fragCoord = vec2(otd_uv.x, 1.0 - otd_uv.y) * U.res.xy;
    mainImage(otd_frag, fragCoord);
}
"#;

/// TouchDesigner vector parameters arrive as `uniform float name` (the x
/// component) or `uniform vecN name`. Those declarations have no binding naga
/// will accept, so each one becomes `#define name (U.pN…)` and the declaration
/// is removed. `bindings` is `(name, "U.pN")`.
pub fn adopt_td_uniforms(source: &str, bindings: &[(&str, &str)]) -> String {
    let mut drop_names = Vec::new();
    let mut header = String::new();
    for (name, slot) in bindings {
        if name.is_empty() || !source.contains(name) {
            continue;
        }
        let expr = match uniform_decl_type(source, name) {
            Some(ty) => {
                drop_names.push(*name);
                format!("({slot}{})", uniform_swizzle(&ty))
            }
            None => format!("({slot})"),
        };
        header.push_str(&format!("#define {name} {expr}\n"));
    }
    let mut body = String::new();
    for line in source.lines() {
        if let Some((_, declared)) = uniform_decl(line) {
            if drop_names.contains(&declared) {
                continue;
            }
        }
        body.push_str(line);
        body.push('\n');
    }
    format!("{header}{body}")
}

fn uniform_decl(line: &str) -> Option<(String, &str)> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    let mut parts: Vec<&str> = trimmed.strip_prefix("uniform ")?.split_whitespace().collect();
    if parts.first().is_some_and(|p| matches!(*p, "highp" | "mediump" | "lowp")) {
        parts.remove(0);
    }
    if parts.len() < 2 {
        return None;
    }
    let ty = parts[0].trim_end_matches(';').to_string();
    let name = parts[1].trim_end_matches([';', ',']);
    if name.is_empty() {
        None
    } else {
        Some((ty, name))
    }
}

fn uniform_decl_type(source: &str, name: &str) -> Option<String> {
    source.lines().find_map(|line| {
        uniform_decl(line).and_then(|(ty, declared)| (declared == name).then_some(ty))
    })
}

fn uniform_swizzle(ty: &str) -> &'static str {
    match ty {
        "float" | "int" | "uint" | "bool" | "double" => ".x",
        "vec2" | "ivec2" | "uvec2" | "bvec2" => ".xy",
        "vec3" | "ivec3" | "uvec3" | "bvec3" => ".xyz",
        _ => "",
    }
}

/// Wrap Shadertoy-style GLSL. A source that already has `void main()` is
/// assembled without the shim. A TouchDesigner pixel shader (`sTD2DInputs`,
/// `vUV`, `TDOutputSwizzle`) gets the TD names instead of Shadertoy's.
pub fn wrap_glsl(source: &str) -> String {
    wrap_glsl_full(source).0
}

/// Wrapped shader plus the `const` arrays lifted into binding 4.
///
/// A `const float K[33][33]` indexed by a loop lives in scratch memory on
/// Metal and is what drops a neighbourhood shader to a few frames a second.
/// The same numbers in a 1×N `R32Float` table use `texelFetch` so the GPU can
/// cache kernel taps across fragments.
pub fn wrap_glsl_full(source: &str) -> (String, Vec<f32>) {
    let (source, table) = hoist_const_arrays(source);
    if is_td_pixel_shader(&source) {
        return (wrap_td_glsl(&source, table.len()), table);
    }
    let mut out = String::from(GLSL_PREAMBLE);
    out.push_str(&const_table_glsl(table.len()));
    // Declaring samplers a shader never uses would still bind them, so only
    // pay for them when the source mentions one.
    let flip_channels = source.contains("iChannel");
    if source.contains("otd_image") || flip_channels {
        out.push_str(GLSL_SAMPLERS);
    }
    // ISF has its own accessors and already flips inside them, so only a
    // source written against Shadertoy's names is rewritten.
    let body = match flip_channels {
        true => std::borrow::Cow::Owned(flip_channel_lookups(&source)),
        false => std::borrow::Cow::Borrowed(&source),
    };
    out.push_str(&body);
    if !source.contains("void main(") {
        out.push_str(GLSL_MAIN);
    }
    (out, table)
}
