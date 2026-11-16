use super::*;

use super::*;

    #[test]
    fn operator_controls_are_a_page_before_common() {
        let mut params = IndexMap::new();
        params.insert("period".into(), Param::float(1.0).with_label("Period"));
        params.insert(
            "format".into(),
            Param::menu("useinput", &["useinput"])
                .with_label("Pixel Format")
                .with_page("Common"),
        );
        let pages = collect_pages(&params, false, "Noise");
        assert_eq!(pages, vec!["Noise".to_string(), "Common".to_string()]);
        assert!(param_on_page(&params["period"], "Noise", true, "Noise"));
        assert!(!param_on_page(&params["period"], "Common", true, "Noise"));
        assert!(param_on_page(&params["format"], "Common", true, "Noise"));
    }

    #[test]
    fn an_unpaged_shader_source_stays_on_the_glsl_page() {
        let mut params = IndexMap::new();
        params.insert("source".into(), Param::str("").with_label("Source"));
        params.insert(
            "pixeldat".into(),
            Param::str("").with_label("Pixel Shader").with_page("GLSL"),
        );
        params.insert(
            "vec0name".into(),
            Param::str("")
                .with_label("Vector 0 Name")
                .with_page("Vectors"),
        );
        params.insert(
            "format".into(),
            Param::str("")
                .with_label("Pixel Format")
                .with_page("Common"),
        );
        let pages = collect_pages(&params, false, "GLSL");
        assert_eq!(pages[0], "GLSL");
        assert_eq!(pages.last().map(String::as_str), Some("Common"));
        assert!(param_on_page(&params["source"], "GLSL", true, "GLSL"));
        assert!(!param_on_page(&params["source"], "Common", true, "GLSL"));
    }
