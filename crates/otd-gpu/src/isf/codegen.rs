use super::import::{IsfError, IsfInput, IsfKind, Slot};

pub fn width(value: &otd_core::Value) -> usize {
    use otd_core::Value::*;
    match value {
        Float(_) | Int(_) | Bool(_) | Select(_) => 1,
        Vec2(_) => 2,
        Vec3(_) | Vec4(_) => 4,
        // Strings are not uniforms — an ISF `image` keeps its wire here.
        Str(_) => 0,
    }
}

/// Assign uniform slots to a list of parameter widths, in declaration order.
///
/// Nothing straddles a vector boundary: a `vec4` starts a vector and a `vec2`
/// starts on an even component. That wastes a component here and there and is
/// worth it — a value split across two uniforms would need the shader to
/// reassemble it, which is exactly the kind of invisible contract that breaks
/// silently later.
///
/// `None` when the parameters do not fit in the vectors available.
pub fn layout(widths: impl IntoIterator<Item = usize>) -> Option<Vec<Slot>> {
    let capacity = crate::ops::PARAM_VECS * 4;
    let mut out = Vec::new();
    let mut cursor = 0usize; // in components, 0..capacity
    for w in widths {
        if w == 0 {
            continue;
        }
        // Align: a vec4 to a vector, a vec2 to an even component.
        let align = if w == 4 { 4 } else { w.min(2) };
        cursor = cursor.next_multiple_of(align);
        if cursor + w > capacity {
            return None;
        }
        out.push(Slot {
            vec: cursor / 4,
            component: cursor % 4,
            width: w,
        });
        cursor += w;
    }
    Some(out)
}

pub(crate) fn number_array(value: Option<&serde_json::Value>, n: usize, fallback: [f64; 4]) -> [f64; 4] {
    let mut out = fallback;
    if let Some(a) = value.and_then(|v| v.as_array()) {
        for (i, v) in a.iter().take(n).enumerate() {
            if let Some(f) = v.as_f64() {
                out[i] = f;
            }
        }
    }
    out
}

/// The GLSL that turns ISF's vocabulary into ours.
///
/// ISF shaders are written against a small set of globals and macros. Rather
/// than rewriting the body — which would mean parsing GLSL — this declares
/// them in terms of what the GLSL TOP already provides.
pub(crate) fn preamble(inputs: &[IsfInput], image_count: usize) -> Result<(String, Vec<String>), IsfError> {
    let mut out = String::from(
        // `__VERSION__` is defined because published ISF filters routinely
        // bracket their varyings in `#if __VERSION__ <= 120`, and naga leaves
        // it undefined — which evaluates to 0, takes the GLSL 1.x branch, and
        // fails the whole shader on `varying` declarations that are illegal
        // at 450. Twenty of the two hundred published filters died on that,
        // none of them for a reason to do with the shader.
        "// --- ISF compatibility ---\n\
         #define __VERSION__ 450\n\
         #define isf_FragNormCoord vec2(otd_uv.x, 1.0 - otd_uv.y)\n\
         #define vv_FragNormCoord isf_FragNormCoord\n\
         #define gl_FragColor otd_frag\n\
         #define TIME iTime\n\
         #define TIMEDELTA iTimeDelta\n\
         #define FRAMEINDEX iFrame\n\
         #define RENDERSIZE iResolution.xy\n\
         #define DATE iDate\n\
         #define PASSINDEX 0\n",
    );

    // ISF's image accessors, in terms of the two texture inputs a GLSL TOP
    // has. A shader asking for more images than that gets the dummy, which
    // reads as transparent black rather than as a compile error.
    //
    // Emitted whether or not the header declares an image, because some
    // published shaders call `IMG_NORM_PIXEL` against a persistent buffer or
    // a target they never listed as an INPUT. Defining a macro nothing uses
    // costs nothing; leaving it out costs the whole shader.
    {
        let _ = image_count;
        out.push_str(
            "#define IMG_NORM_PIXEL(img, uv) texture(img, vec2((uv).x, 1.0 - (uv).y))\n\
             #define IMG_THIS_NORM_PIXEL(img) IMG_NORM_PIXEL(img, isf_FragNormCoord)\n\
             #define IMG_PIXEL(img, px) IMG_NORM_PIXEL(img, (px) / RENDERSIZE)\n\
             #define IMG_THIS_PIXEL(img) IMG_THIS_NORM_PIXEL(img)\n\
             #define IMG_SIZE(img) RENDERSIZE\n",
        );
    }

    // Each declared input becomes a uniform the engine fills. The slots come
    // from the same `layout` the engine packs with, so the two cannot drift.
    let slots = layout(inputs.iter().map(|i| i.kind.width())).ok_or(IsfError::TooManyInputs)?;

    let mut assignments: Vec<String> = Vec::new();
    let mut image_index = 0;
    let mut slot_index = 0;
    for input in inputs {
        if input.kind == IsfKind::Image {
            // Images are the GLSL TOP's own texture inputs, not uniforms.
            out.push_str(&format!(
                "#define {} otd_image{}\n",
                input.name, image_index
            ));
            image_index += 1;
            continue;
        }
        let slot = slots[slot_index];
        slot_index += 1;
        let u = format!("U.p{}{}", slot.vec, slot.swizzle());
        // A global, not a `#define`.
        //
        // A macro rewrites the name *everywhere*, including where the shader
        // declares something of its own with that name — and ISF inputs are
        // called `size`, `color`, `angle`, `level`, so the collision is not
        // exotic. `Heart.fs` declares `bool inHeart(vec2 p, vec2 c, float
        // size)`, the macro turns that parameter into `float U.p0.x`, and the
        // shader dies on a syntax error nowhere near anything its author
        // wrote.
        //
        // A global declaration is the same substitution where the author
        // meant one, and correct GLSL shadowing where they did not: the
        // parameter wins inside the function, exactly as intended. The value
        // is assigned in `main` because a GLSL global cannot be initialised
        // from a uniform.
        let (ty, expression) = match input.kind {
            IsfKind::Bool => ("bool", format!("{u} > 0.5")),
            IsfKind::Long => ("int", format!("int({u})")),
            IsfKind::Color => ("vec4", u.clone()),
            IsfKind::Point2d => ("vec2", u.clone()),
            _ => ("float", u.clone()),
        };
        out.push_str(&format!("{ty} {};\n", input.name));
        assignments.push(format!("    {} = {expression};\n", input.name));
    }
    out.push_str("// --- end ISF compatibility ---\n");
    Ok((out, assignments))
}

/// Wrap an ISF body so its inputs are assigned before it runs.
///
/// The body brings its own `void main()`, so the assignments cannot simply be
/// prepended — they have to happen *inside* it, before the first line that
/// might read one. Renaming the body's entry point and calling it from ours
/// is the least invasive way to get there: no GLSL parsing, and the body is
/// otherwise untouched.
pub(crate) fn wrap_entry_point(body: &str, assignments: &[String]) -> String {
    if assignments.is_empty() {
        return body.to_string();
    }
    let renamed = rename_main(body);
    if renamed == body {
        // No `void main` to rename — the shader is a bare body and the GLSL
        // TOP will supply the entry point, so the assignments can go first.
        return format!("{}{body}", assignments.concat());
    }
    format!(
        "{renamed}\n\nvoid main() {{\n{}    isf_main();\n}}\n",
        assignments.concat()
    )
}

/// Rename the body's `void main(` to `void isf_main(`, once.
fn rename_main(body: &str) -> String {
    let mut out = String::with_capacity(body.len() + 8);
    let mut rest = body;
    let mut done = false;
    while let Some(i) = rest.find("main") {
        let (before, after) = rest.split_at(i);
        let following = &after[4..];
        // A declaration or definition: `main` followed by optional space and
        // an open paren, and preceded by something that is not an identifier
        // character, so `mainImage` and `domain` are left alone.
        let is_call_site = following.trim_start().starts_with('(');
        let preceded_ok = before
            .chars()
            .last()
            .map(|c| !c.is_alphanumeric() && c != '_')
            .unwrap_or(true);
        out.push_str(before);
        if !done && is_call_site && preceded_ok {
            out.push_str("isf_main");
            done = true;
        } else {
            out.push_str("main");
        }
        rest = following;
    }
    out.push_str(rest);
    out
}
