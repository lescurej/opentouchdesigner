use super::*;
use otd_core::{Graph, ParamMode, Value};
use otd_engine::registry;

    fn add_op(g: &mut Graph, op: &str, name: &str) -> otd_core::NodeId {
        let reg = registry();
        let def = reg.get(op).unwrap_or_else(|| panic!("no op `{op}`"));
        let root = g.root();
        g.create(root, def, Some(name)).unwrap()
    }

    #[test]
    fn lfo_frequency_mode_16_is_value_not_expression() {
        let p = parse_parm_line("frequency 16 0.11 1/5").unwrap();
        assert_eq!(p.td_mode, 16);
        assert_eq!(p.expression.as_deref(), Some("1/5"));
        assert!(matches!(p.constant, Some(Value::Float(f)) if (f - 0.11).abs() < 1e-6));

        let mut g = Graph::new();
        let id = add_op(&mut g, "lfoCHOP", "lfo1");
        apply_parsed_parms(&mut g, id, "lfoCHOP", &[p], &mut vec![]);
        let param = g.node(id).params.get("frequency").unwrap();
        assert_eq!(param.mode, ParamMode::Constant);
        assert!((param.value.as_f64() - 0.11).abs() < 1e-6);
        assert!(param.expression.is_empty());
    }

    #[test]
    fn lfo_amp_maps_to_amplitude() {
        let p = parse_parm_line("amp 0 18").unwrap();
        let mut g = Graph::new();
        let id = add_op(&mut g, "lfoCHOP", "lfo1");
        apply_parsed_parms(&mut g, id, "lfoCHOP", &[p], &mut vec![]);
        assert!((g.node(id).params["amplitude"].value.as_f64() - 18.0).abs() < 1e-6);
    }

    #[test]
    fn fan_channame_mode_32() {
        let p = parse_parm_line("channame 32 bulb[1-18]").unwrap();
        assert_eq!(p.constant, Some(Value::Str("bulb[1-18]".into())));
    }

    #[test]
    fn rename_dmx_f_string_expression() {
        let p = parse_parm_line(
            "renameto 49 f'' f'[{parent(2).par.Dmxstartaddress}-{parent(2).par.Dmxstartaddress+20}]'",
        )
        .unwrap();
        assert_eq!(
            p.expression.as_deref(),
            Some("f'[{parent(2).par.Dmxstartaddress}-{parent(2).par.Dmxstartaddress+20}]'")
        );
        let mut g = Graph::new();
        let id = add_op(&mut g, "renameCHOP", "rename1");
        apply_parsed_parms(&mut g, id, "renameCHOP", &[p], &mut vec![]);
        let param = &g.node(id).params["to"];
        assert_eq!(param.mode, ParamMode::Expression);
        assert!(param.expression.starts_with("f'"));
    }

    #[test]
    fn chopexecute_channel_maps_to_channels() {
        let p = parse_parm_line("channel 0 Showlabel").unwrap();
        assert_eq!(map_parm_key("chopexecuteDAT", &p.key), "channels");
        let mut g = Graph::new();
        let id = add_op(&mut g, "chopexecuteDAT", "chopexec1");
        apply_parsed_parms(&mut g, id, "chopexecuteDAT", &[p], &mut vec![]);
        assert_eq!(g.node(id).params["channels"].value.as_str(), "Showlabel");
    }

    #[test]
    fn geotext_text_mode_17_expression() {
        let p = parse_parm_line("text 17 parent().name parent().name").unwrap();
        assert_eq!(p.expression.as_deref(), Some("parent().name"));
        let mut g = Graph::new();
        let def = otd_engine::registry().get("geotextCOMP").unwrap().clone();
        let root = g.root();
        let id = g.create(root, &def, Some("geotext1")).unwrap();
        apply_parsed_parms(&mut g, id, "geotextCOMP", &[p], &mut vec![]);
        let param = g.node(id).params.get("text").unwrap();
        assert_eq!(param.mode, ParamMode::Expression);
        assert_eq!(param.expression, "parent().name");
    }

    #[test]
    fn rename_mode_49_expression_on_to() {
        let p = parse_parm_line("renameto 49 f'{parent().name}/*' f'{parent().name}/*'").unwrap();
        assert_eq!(p.expression.as_deref(), Some("f'{parent().name}/*'"));
        let mut g = Graph::new();
        let id = add_op(&mut g, "renameCHOP", "rename1");
        apply_parsed_parms(&mut g, id, "renameCHOP", &[p], &mut vec![]);
        let param = g.node(id).params.get("to").unwrap();
        assert_eq!(param.mode, ParamMode::Expression);
        assert_eq!(param.expression, "f'{parent().name}/*'");
    }

    #[test]
    fn td_parm_aliases_land_on_real_parameters() {
        let mut g = Graph::new();
        let glsl = add_op(&mut g, "glslTOP", "glsl1");
        let noise = add_op(&mut g, "noiseTOP", "noise1");
        let reorder = add_op(&mut g, "reorderTOP", "reorder1");
        let feedback = add_op(&mut g, "feedbackTOP", "feedback1");
        let blur = add_op(&mut g, "blurTOP", "blur1");
        let text = add_op(&mut g, "textDAT", "glsl1_pixel");

        apply_parsed_parms(
            &mut g,
            glsl,
            "glslTOP",
            &parse_parm_block(
                "?\npixeldat 0 glsl1_pixel\nvec0name 0 uTime\nvec0valuex 17 0.5 absTime.seconds\n\
                 const1name 0 \"\"\nconst1value 0 1\ninputfiltertype 0 nearest\n?\n",
            ),
            &mut vec![],
        );
        apply_parsed_parms(
            &mut g,
            noise,
            "noiseTOP",
            &parse_parm_block(
                "?\nharmon 0 0\nexp 0 0.77\ngain 0 1.44\nseed 0 3354\nspread 0 15.2\n?\n",
            ),
            &mut vec![],
        );
        apply_parsed_parms(
            &mut g,
            reorder,
            "reorderTOP",
            &parse_parm_block(
                "?\noutputredchan 0 alpha\noutputgreenchan 0 alpha\noutputbluechan 0 alpha\n?\n",
            ),
            &mut vec![],
        );
        apply_parsed_parms(
            &mut g,
            feedback,
            "feedbackTOP",
            &parse_parm_block("?\ntop 0 glsl1\ninputfiltertype 0 nearest\n?\n"),
            &mut vec![],
        );
        apply_parsed_parms(
            &mut g,
            blur,
            "blurTOP",
            &parse_parm_block("?\npreshrink 0 2\n?\n"),
            &mut vec![],
        );
        apply_parsed_parms(
            &mut g,
            text,
            "textDAT",
            &parse_parm_block("?\nlanguage 0 glsl\nextension 0 frag\n?\n"),
            &mut vec![],
        );

        assert_eq!(
            g.node(glsl).params["pixeldat"].value.as_str(),
            "glsl1_pixel"
        );
        assert_eq!(g.node(glsl).params["vec0name"].value.as_str(), "uTime");
        assert_eq!(
            g.node(glsl).params["vec0valuex"].mode,
            ParamMode::Expression
        );
        assert_eq!(g.node(glsl).params["const1name"].value.as_str(), "");
        assert_eq!(g.node(glsl).params["filtertype"].value.as_str(), "nearest");
        assert_eq!(g.node(noise).params["harmonics"].value.as_i64(), 0);
        assert!((g.node(noise).params["exponent"].value.as_f64() - 0.77).abs() < 1e-6);
        assert!((g.node(noise).params["gain"].value.as_f64() - 1.44).abs() < 1e-6);
        assert!((g.node(noise).params["roughness"].value.as_f64() - 0.5).abs() < 1e-6);
        assert!((g.node(noise).params["seed"].value.as_f64() - 3354.0).abs() < 1e-6);
        assert_eq!(g.node(reorder).params["red"].value.as_str(), "input1 a");
        assert_eq!(g.node(feedback).params["target"].value.as_str(), "glsl1");
        assert_eq!(
            g.node(feedback).params["filtertype"].value.as_str(),
            "nearest"
        );
        assert_eq!(g.node(blur).params["preshrink"].value.as_i64(), 2);
        assert_eq!(g.node(text).params["language"].value.as_str(), "glsl");
        assert_eq!(g.node(text).params["extension"].value.as_str(), "frag");
    }

    #[test]
    fn select_chops_constant_path() {
        let p = parse_parm_line("chops 0 ../oscin1").unwrap();
        assert_eq!(p.constant, Some(Value::Str("../oscin1".into())));
    }

    #[test]
    fn fan_quantize_imported() {
        let p = parse_parm_line("quantize 0 off").unwrap();
        let mut g = Graph::new();
        let id = add_op(&mut g, "fanCHOP", "fan1");
        apply_parsed_parms(&mut g, id, "fanCHOP", &[p], &mut vec![]);
        assert_eq!(g.node(id).params["quantize"].value.as_str(), "off");
        assert_eq!(g.node(id).params["quantize"].menu_index("off"), 0);
        assert_eq!(g.node(id).params["quantize"].mode, ParamMode::Constant);
    }
