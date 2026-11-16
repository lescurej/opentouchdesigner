use super::*;

use super::*;

    const SIMPLE: &str = r#"/*{
        "DESCRIPTION": "A test effect",
        "CREDIT": "nobody",
        "CATEGORIES": ["Color Effect"],
        "INPUTS": [
            { "NAME": "level", "TYPE": "float", "DEFAULT": 0.5, "MIN": 0.0, "MAX": 2.0 },
            { "NAME": "tint", "TYPE": "color", "DEFAULT": [1.0, 0.5, 0.0, 1.0] },
            { "NAME": "flip", "TYPE": "bool", "DEFAULT": 1 }
        ]
    }*/

void main() {
    gl_FragColor = vec4(isf_FragNormCoord, level, 1.0) * tint;
}
"#;

    #[test]
    fn the_header_becomes_parameters() {
        let isf = import(SIMPLE).unwrap();
        assert_eq!(isf.description, "A test effect");
        assert_eq!(isf.categories, vec!["Color Effect"]);

        let level = &isf.params["level"];
        assert_eq!(level.value, otd_core::Value::Float(0.5));
        assert_eq!(level.range, Some((0.0, 2.0)));

        assert_eq!(
            isf.params["tint"].value,
            otd_core::Value::Vec4([1.0, 0.5, 0.0, 1.0])
        );
        assert_eq!(isf.params["flip"].value, otd_core::Value::Bool(true));
    }

    #[test]
    fn the_body_is_kept_and_the_vocabulary_is_defined() {
        let isf = import(SIMPLE).unwrap();
        assert!(isf.source.contains("void main()"), "the body survived");
        assert!(isf.source.contains("#define isf_FragNormCoord"));
        assert!(isf.source.contains("#define TIME iTime"));
        // Inputs map to uniform slots in declaration order. `level` takes one
        // component; `tint` is a vec4 so it starts a fresh vector; `flip`
        // follows it.
        assert!(
            isf.source.contains("float level;") && isf.source.contains("level = U.p0.x;"),
            "{}",
            isf.source
        );
        assert!(
            isf.source.contains("vec4 tint;") && isf.source.contains("tint = U.p1;"),
            "{}",
            isf.source
        );
        assert!(
            isf.source.contains("bool flip;") && isf.source.contains("flip = U.p2.x > 0.5;"),
            "{}",
            isf.source
        );
    }

    #[test]
    fn scalars_share_a_vector_instead_of_each_taking_one() {
        let src = r#"/*{ "INPUTS": [
            { "NAME": "a", "TYPE": "float" },
            { "NAME": "b", "TYPE": "float" },
            { "NAME": "c", "TYPE": "float" },
            { "NAME": "d", "TYPE": "float" },
            { "NAME": "e", "TYPE": "float" }
        ] }*/
void main() {}"#;
        let isf = import(src).unwrap();
        // Four dials fit one vec4; the fifth starts the next. A shader with
        // eight knobs is entirely ordinary, and would not fit at all if each
        // one took a whole vector.
        for (name, u) in [
            ("a", "U.p0.x"),
            ("b", "U.p0.y"),
            ("c", "U.p0.z"),
            ("d", "U.p0.w"),
            ("e", "U.p1.x"),
        ] {
            assert!(
                isf.source.contains(&format!("{name} = {u};")),
                "{name} -> {u}\n{}",
                isf.source
            );
        }
    }

    #[test]
    fn nothing_straddles_a_vector_boundary() {
        // x, then a point2D: the point cannot start at component 1, so it
        // aligns to 2 and leaves a hole.
        let slots = layout([1, 2, 4, 1]).unwrap();
        assert_eq!(
            slots[0],
            Slot {
                vec: 0,
                component: 0,
                width: 1
            }
        );
        assert_eq!(
            slots[1],
            Slot {
                vec: 0,
                component: 2,
                width: 2
            }
        );
        assert_eq!(
            slots[2],
            Slot {
                vec: 1,
                component: 0,
                width: 4
            }
        );
        assert_eq!(
            slots[3],
            Slot {
                vec: 2,
                component: 0,
                width: 1
            }
        );
        assert_eq!(slots[1].swizzle(), ".zw");
        assert_eq!(slots[2].swizzle(), "");
    }

    #[test]
    fn a_shader_that_does_not_fit_is_refused_rather_than_aliased() {
        // One dial past the end does not fit. Silently folding the extras onto
        // the last slot would look like it worked.
        let capacity = crate::ops::PARAM_VECS * 4;
        assert_eq!(layout(std::iter::repeat_n(1, capacity + 1)), None);
        assert!(layout(std::iter::repeat_n(1, capacity)).is_some());

        let colors = |n: usize| {
            let inputs: String = (0..n)
                .map(|i| format!(r#"{{ "NAME": "c{i}", "TYPE": "color" }}"#))
                .collect::<Vec<_>>()
                .join(",");
            format!("/*{{ \"INPUTS\": [{inputs}] }}*/\nvoid main() {{}}")
        };
        assert_eq!(
            import(&colors(crate::ops::PARAM_VECS + 1)).unwrap_err(),
            IsfError::TooManyInputs
        );

        // Five colours were the old ceiling, and the published corpus has
        // sixteen shaders past it — `Corner Colors` is one, and it is the
        // whole reason the block is twelve vectors wide rather than four.
        assert!(import(&colors(5)).is_ok(), "five colours have to fit now");
        assert!(import(&colors(crate::ops::PARAM_VECS)).is_ok());
    }

    /// A parameter in the widened part of the block reaches the shader by the
    /// same name it had in the header.
    #[test]
    fn an_input_past_the_first_four_vectors_gets_a_slot() {
        let inputs: String = (0..6)
            .map(|i| format!(r#"{{ "NAME": "c{i}", "TYPE": "color" }}"#))
            .collect::<Vec<_>>()
            .join(",");
        let src = format!("/*{{ \"INPUTS\": [{inputs}] }}*/\nvoid main() {{}}");
        let isf = import(&src).unwrap();
        assert!(isf.source.contains("c4 = U.p4;"), "{}", isf.source);
        assert!(isf.source.contains("c5 = U.p5;"));
        crate::shader::validate_glsl(&crate::shader::wrap_glsl(&isf.source))
            .expect("a six-colour shader should compile");
    }

    #[test]
    fn image_inputs_become_texture_inputs_not_parameters() {
        let src = r#"/*{
            "INPUTS": [
                { "NAME": "inputImage", "TYPE": "image" },
                { "NAME": "amount", "TYPE": "float", "DEFAULT": 1.0 }
            ]
        }*/
void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage) * amount; }"#;
        let isf = import(src).unwrap();
        assert!(
            !isf.params.contains_key("inputImage"),
            "an image is a wire, not a dial"
        );
        assert!(isf.params.contains_key("amount"));
        assert!(isf.source.contains("#define inputImage otd_image0"));
        assert!(isf.source.contains("#define IMG_THIS_PIXEL"));
        // The float still gets slot 0: images do not consume uniform slots.
        assert!(isf.source.contains("amount = U.p0.x;"));
    }

    #[test]
    fn a_long_input_becomes_a_menu() {
        let src = r#"/*{
            "INPUTS": [
                { "NAME": "mode", "TYPE": "long",
                  "LABELS": ["add", "multiply"], "VALUES": [0, 1], "DEFAULT": 0 }
            ]
        }*/
void main() { gl_FragColor = vec4(float(mode)); }"#;
        let isf = import(src).unwrap();
        let menu = isf.params["mode"].menu.as_deref().unwrap();
        assert_eq!(menu[0].option, "add");
        assert_eq!(menu[0].value, "0");
        assert_eq!(menu[1].option, "multiply");
        assert_eq!(menu[1].value, "1");
        assert_eq!(
            isf.params["mode"].value,
            otd_core::Value::Select("0".into())
        );
        assert!(isf.source.contains("mode = int(U.p0.x);"));
    }

    #[test]
    fn a_file_without_a_header_is_refused_clearly() {
        assert_eq!(import("void main() {}").unwrap_err(), IsfError::NoHeader);
        assert!(matches!(
            import("/*{ not json }*/ void main() {}").unwrap_err(),
            IsfError::BadJson(_)
        ));
    }

    #[test]
    fn multi_pass_is_refused_rather_than_half_run() {
        let src = r#"/*{
            "PASSES": [ { "TARGET": "a" }, { } ]
        }*/
void main() {}"#;
        assert_eq!(import(src).unwrap_err(), IsfError::MultiPass);
    }

    /// Compile what the importer produced, the way the node would.
    fn compiles(isf: &Isf) -> Result<(), String> {
        crate::shader::validate_glsl(&crate::shader::wrap_glsl(&isf.source))
    }

    #[test]
    fn an_input_named_like_a_local_does_not_rewrite_the_local() {
        // Straight out of the published collection: `Heart.fs` declares an
        // input called `size` and also `bool inHeart(vec2 p, vec2 c, float
        // size)`. As a `#define` the parameter became `float U.p0.x` and the
        // shader died on a syntax error nowhere near anything its author
        // wrote. As a global it shadows correctly, which is what GLSL is for.
        // Eleven of the two hundred published filters failed this way.
        let src = r#"/*{
            "INPUTS": [ { "NAME": "size", "TYPE": "float", "DEFAULT": 0.5 } ]
        }*/
        bool big(float size) { return size > 0.5; }
        void main() {
            gl_FragColor = vec4(vec3(big(size) ? 1.0 : 0.0), 1.0);
        }"#;
        let isf = import(src).unwrap();
        assert!(
            isf.source.contains("float size;"),
            "the input should be a global:\n{}",
            isf.source
        );
        assert!(
            isf.source.contains("bool big(float size)"),
            "the function parameter must survive untouched:\n{}",
            isf.source
        );
        if let Err(e) = compiles(&isf) {
            panic!("{e}\n---\n{}", isf.source);
        }
    }

    #[test]
    fn a_shader_guarded_on_the_glsl_version_takes_the_modern_branch() {
        // Published filters bracket their varyings in `#if __VERSION__ <=
        // 120`. Undefined, that is zero, which is <= 120 — so the legacy
        // branch is taken and a pile of `varying` declarations that are
        // illegal at 450 fail the shader. Twenty filters died on this.
        let src = r#"/*{
            "INPUTS": [ { "NAME": "level", "TYPE": "float" } ]
        }*/
        #if __VERSION__ <= 120
        varying vec2 legacy_coord;
        #endif
        void main() {
            gl_FragColor = vec4(vec3(level), 1.0);
        }"#;
        let isf = import(src).unwrap();
        if let Err(e) = compiles(&isf) {
            panic!(
                "the modern branch should have been taken: {e}\n---\n{}",
                isf.source
            );
        }
    }

    #[test]
    fn the_image_accessors_exist_even_with_no_image_input_declared() {
        // Some published shaders reach for IMG_NORM_PIXEL against something
        // they never listed in INPUTS. A macro nothing uses costs nothing;
        // leaving it out costs the whole shader.
        let src = r#"/*{ "INPUTS": [] }*/
        void main() { gl_FragColor = IMG_NORM_PIXEL(otd_image0, isf_FragNormCoord); }"#;
        let isf = import(src).unwrap();
        assert!(isf.source.contains("#define IMG_NORM_PIXEL"));
        compiles(&isf).unwrap();
    }

    #[test]
    fn renaming_the_entry_point_leaves_similar_names_alone() {
        // `mainImage` and `domain` both contain `main`, and a careless rename
        // would break a shader that has neither problem.
        assert_eq!(rename_main("void main() {}"), "void isf_main() {}");
        assert_eq!(
            rename_main("float domain(float x) { return x; }\nvoid main(void) {}"),
            "float domain(float x) { return x; }\nvoid isf_main(void) {}"
        );
        assert_eq!(
            rename_main("void mainImage(out vec4 c) {}"),
            "void mainImage(out vec4 c) {}",
            "mainImage is not the entry point and must not be renamed"
        );
        // Only the first one, so a shader that mentions main() in a comment
        // after its definition is unaffected.
        assert_eq!(
            rename_main("void main() {}\n// main() again"),
            "void isf_main() {}\n// main() again"
        );
    }
