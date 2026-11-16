use super::*;

use super::*;

    #[test]
    fn a_wgsl_body_is_wrapped_and_compiles() {
        let src = "return vec4<f32>(in.uv, 0.0, 1.0);";
        validate_wgsl(&wrap_wgsl(src)).expect("wrapped body should compile");
    }

    #[test]
    fn a_full_wgsl_shader_is_passed_through() {
        let src =
            "@fragment\nfn fs_main(in: VOut) -> @location(0) vec4<f32> { return sample0(in.uv); }";
        validate_wgsl(&wrap_wgsl(src)).expect("full shader should compile");
    }

    #[test]
    fn a_wgsl_typo_reports_an_error_rather_than_panicking() {
        let err = validate_wgsl(&wrap_wgsl("return vec4<f32>(nope);")).unwrap_err();
        assert!(!err.is_empty());
        assert!(err.len() < 400, "error should be a summary, got: {err}");
    }

    #[test]
    fn shadertoy_glsl_compiles() {
        let src = "void mainImage(out vec4 fragColor, in vec2 fragCoord) {\n\
                   vec2 uv = fragCoord / iResolution.xy;\n\
                   fragColor = vec4(uv, 0.5 + 0.5 * sin(iTime), 1.0);\n\
                   }";
        if let Err(e) = validate_glsl(&wrap_glsl(src)) {
            panic!("Shadertoy-style GLSL failed to compile: {e}");
        }
    }

    /// The uniform block is declared three times — the Rust struct, the WGSL
    /// prelude and the GLSL one — and a shader reading `U.p9` from a block
    /// that stops at `p3` is a validation error a long way from its cause.
    #[test]
    fn both_uniform_blocks_declare_every_parameter_vector() {
        let last = crate::ops::PARAM_VECS - 1;
        for (language, source) in [
            ("GLSL", GLSL_PREAMBLE),
            ("TD", TD_PREAMBLE),
            ("WGSL", COMMON_WGSL),
        ] {
            assert!(
                source.contains(&format!("p{last}")),
                "the {language} uniform block stops short of p{last}"
            );
            assert!(
                !source.contains(&format!("p{}", last + 1)),
                "the {language} uniform block declares more than ops::PARAM_VECS vectors"
            );
        }
    }

    #[test]
    fn the_noise_shader_still_compiles() {
        let src = include_str!("shaders/noise.wgsl");
        let full = format!("{}\n{src}", crate::ops::COMMON_WGSL);
        validate_wgsl(&full).expect("noise shader");
    }

    #[test]
    fn the_rewritten_top_shaders_compile() {
        for (name, src) in [
            ("blur", include_str!("shaders/blur.wgsl")),
            ("threshold", include_str!("shaders/threshold.wgsl")),
            ("switch", include_str!("shaders/switch.wgsl")),
            ("analyze", include_str!("shaders/analyze.wgsl")),
        ] {
            let full = format!("{}\n{src}", crate::ops::COMMON_WGSL);
            validate_wgsl(&full).unwrap_or_else(|e| panic!("{name} shader failed to compile: {e}"));
        }
    }

    #[test]
    fn a_td_float_uniform_binds_to_the_vector_x_and_compiles() {
        let src = "\
layout(location = 0) out vec4 fragColor;
uniform float uIntensity;
uniform float uDisplayMode;
void main() {
    vec4 t = texture(sTD2DInputs[0], vUV.st);
    float m = max(0.0, uDisplayMode);
    fragColor = TDOutputSwizzle(vec4(t.r * uIntensity, m, 0.0, 1.0));
}
";
        let bound = adopt_td_uniforms(src, &[("uIntensity", "U.p0"), ("uDisplayMode", "U.p1")]);
        assert!(bound.contains("#define uIntensity (U.p0.x)"));
        assert!(bound.contains("#define uDisplayMode (U.p1.x)"));
        assert!(!bound.contains("uniform float"));
        let full = wrap_glsl(&bound);
        validate_glsl(&full).expect("TouchDesigner float uniforms should compile");
    }

    #[test]
    fn a_const_kernel_is_a_buffer_load() {
        let src = "\
layout(location = 0) out vec4 fragColor;
#define N 2
const float K[N][N] = float[N][N](float[N](1.0, 0.0), float[N](0.0, 4.0));
const int I[N] = int[N](3, 5);
void main() {
    int y = 1;
    int x = 1;
    fragColor = TDOutputSwizzle(vec4(K[y][x], float(I[0]), 0.0, 1.0));
}
";
        let (full, table) = wrap_glsl_full(src);
        assert_eq!(table, vec![1.0, 0.0, 0.0, 4.0, 3.0, 5.0]);
        assert!(full.contains("otdK(0 + (y) * 2 + (x))"), "{full}");
        assert!(full.contains("int(otdK(4 + (0)))"), "{full}");
        assert!(!full.contains("const float K"));
        validate_glsl(&full).unwrap_or_else(|e| panic!("{e}\n{full}"));
    }

    #[test]
    fn a_td_output_resolution_compiles() {
        let src = "\
layout(location = 0) out vec4 fragColor;
uniform float uSeed;
void main() {
    ivec2 res = ivec2(uTDOutputInfo.res.zw);
    fragColor = TDOutputSwizzle(vec4(float(res.x) * uSeed, 0.0, 0.0, 1.0));
}
";
        let bound = adopt_td_uniforms(src, &[("uSeed", "U.p0")]);
        let full = wrap_glsl(&bound);
        assert!(full.contains("uTDOutputInfo") == false);
        validate_glsl(&full).expect("uTDOutputInfo should compile");
    }

    #[test]
    fn a_touchdesigner_pixel_shader_compiles() {
        let src = "\
layout(location = 0) out vec4 fragColor;
void readChemicals(vec4 s, out float u, out float v) {
    u = s.r;
    v = s.g;
}
void main() {
    vec2 px = vec2(uTD2DInfos[0].res.s, uTD2DInfos[0].res.t);
    vec2 uv = vUV.st;
    float uC;
    float vC;
    readChemicals(texture(sTD2DInputs[0], fract(uv + px)), uC, vC);
    fragColor = TDOutputSwizzle(vec4(uC, vC, gl_FragCoord.x, 1.0));
}
";
        let full = wrap_glsl(src);
        assert!(full.contains("sampler2D(otd_tex0, otd_samp)"));
        assert!(full.contains("1.0 - otd_uv.y"));
        assert!(full.contains("tdFragCoord()"));
        assert!(!full.contains("out vec4 otd_frag"));
        validate_glsl(&full).expect("TouchDesigner pixel shader should compile");
    }

    #[test]
    fn a_glsl_typo_reports_an_error() {
        let src = "void mainImage(out vec4 fragColor, in vec2 fragCoord) { fragColor = nope; }";
        assert!(validate_glsl(&wrap_glsl(src)).is_err());
    }
