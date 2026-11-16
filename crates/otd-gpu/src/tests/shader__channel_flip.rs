use super::*;

    #[test]
    fn our_channels_are_flipped_and_nothing_else_is() {
        let out = flip_channel_lookups(
            "texture(iChannel0, uv) + texture(iChannel1, vec2(u.x, f(a, b))) \
             + textureLod(iChannel0, uv, 0.0) + texture(other, uv) \
             + texture(iChannel0, uv, 0.5) + mytexture(iChannel0, uv)",
        );
        assert!(out.contains("texture(iChannel0, vec2((uv).x, 1.0 - (uv).y))"));
        // A comma inside the coordinate expression is not an argument break.
        assert!(out.contains(
            "texture(iChannel1, vec2((vec2(u.x, f(a, b))).x, 1.0 - (vec2(u.x, f(a, b))).y))"
        ));
        // Left alone: a different function, a different sampler, the
        // three-argument form, and an identifier that merely ends in it.
        assert!(out.contains("textureLod(iChannel0, uv, 0.0)"));
        assert!(out.contains("texture(other, uv)"));
        assert!(out.contains("texture(iChannel0, uv, 0.5)"));
        assert!(out.contains("mytexture(iChannel0, uv)"));
    }

    #[test]
    fn a_shadertoy_passthrough_compiles_after_the_rewrite() {
        let src = "void mainImage(out vec4 fragColor, in vec2 fragCoord) {\n\
                   vec2 uv = fragCoord / iResolution.xy;\n\
                   fragColor = texture(iChannel0, uv);\n}";
        let full = wrap_glsl(src);
        assert!(full.contains("vec2((uv).x, 1.0 - (uv).y)"));
        validate_glsl(&full).expect("the rewritten passthrough must still compile");
    }

    #[test]
    fn an_isf_source_is_left_alone() {
        // ISF flips inside IMG_NORM_PIXEL already; doing it again is the same
        // bug the other way up.
        let src = "void main() { gl_FragColor = texture(otd_image0, isf_FragNormCoord); }";
        let full = wrap_glsl(src);
        assert!(full.contains("texture(otd_image0, isf_FragNormCoord)"));
        assert!(!full.contains("1.0 - (isf_FragNormCoord).y"));
    }
