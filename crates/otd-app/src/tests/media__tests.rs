use super::*;

use super::*;

    #[test]
    fn extensions_choose_the_operator() {
        assert_eq!(classify(Path::new("a/b/clip.MP4")), DropKind::Movie);
        assert_eq!(classify(Path::new("still.png")), DropKind::Image);
        assert_eq!(classify(Path::new("loop.wav")), DropKind::Audio);
        assert_eq!(classify(Path::new("show.otd")), DropKind::Project);
        assert_eq!(classify(Path::new("thing.otdc")), DropKind::Component);
        assert_eq!(classify(Path::new("warp.fs")), DropKind::Isf);
        assert_eq!(classify(Path::new("warp.wgsl")), DropKind::Shader("wgsl"));
        assert_eq!(classify(Path::new("cues.csv")), DropKind::Table("comma"));
        assert_eq!(classify(Path::new("notes.md")), DropKind::Text);
        assert_eq!(classify(Path::new("Inter.ttf")), DropKind::Font);
        assert_eq!(classify(Path::new("net.tox")), DropKind::TouchDesigner);
        assert_eq!(classify(Path::new("show.TOE")), DropKind::TouchDesigner);
        assert_eq!(classify(Path::new("archive.zip")), DropKind::Unknown);
        assert_eq!(classify(Path::new("no-extension")), DropKind::Unknown);
    }

    #[test]
    fn a_file_name_becomes_a_legal_operator_name() {
        assert_eq!(
            sanitise("My Clip (final)").as_deref(),
            Some("my_clip_final")
        );
        assert_eq!(sanitise("beach").as_deref(), Some("beach"));
        // A name that would not parse as one is better skipped than mangled:
        // the auto-numbered `movie1` is always valid.
        assert_eq!(sanitise("2024-06-11"), None);
        assert_eq!(sanitise("____"), None);
    }

    #[test]
    fn a_long_file_name_is_cut_to_something_a_node_header_can_show() {
        // The name an image generator gives you, which overflowed the node.
        assert_eq!(
            sanitise("chatgpt_image_aug_5_2026_11_59_18_pm").as_deref(),
            Some("chatgpt_image_aug_5")
        );
        // A real one, off a phone: spaces, an emoji and two hashtags, and
        // 29 characters of name once they are gone.
        assert_eq!(
            sanitise("The follower 🧿 #touchdesigner #IK").as_deref(),
            Some("the_follower")
        );
        // No boundary near the end to cut at: a hard cut beats no name.
        assert_eq!(
            sanitise("averyveryverylongsinglewordname").as_deref(),
            Some("averyveryverylongsin")
        );
        for name in [
            "chatgpt_image_aug_5_2026_11_59_18_pm",
            "averyveryverylongsinglewordname",
        ] {
            assert!(sanitise(name).unwrap().len() <= MAX_NAME);
        }
    }

    /// The drop handler sets parameters by name on operators it names. Both
    /// are strings, and a rename on either side would otherwise turn a drop
    /// into a node that quietly plays nothing.
    #[test]
    fn every_operator_a_drop_targets_takes_the_parameters_it_is_given() {
        let registry = otd_engine::registry();
        let targets: &[(&str, &[&str])] = &[
            (otd_gpu::ops::MOVIE_IN, &["file"]),
            (otd_gpu::ops::VIDEO_DEVICE_IN, &[]),
            (otd_gpu::ops::GLSL, &["language", "source"]),
            (otd_gpu::ops::TEXT, &["font", "text"]),
            (otd_chop::io::AUDIO_FILE, &["file"]),
            (otd_dat::ops::TABLE, &["text", "delimiter"]),
            (otd_dat::ops::TEXT, &["text"]),
        ];
        for (type_name, keys) in targets {
            let def = registry
                .get(type_name)
                .unwrap_or_else(|| panic!("{type_name} is not registered"));
            let params = (def.params)();
            for key in *keys {
                assert!(
                    params.contains_key(*key),
                    "{type_name} has no `{key}` parameter for a dropped file to fill"
                );
            }
        }
    }

    #[test]
    fn add_operator_menu_lists_every_registered_op_by_family() {
        let registry = otd_engine::registry();
        for family in ADD_OP_FAMILIES {
            let count = registry.iter().filter(|d| d.family == family).count();
            assert!(count > 0, "no {family:?} operators in registry");
        }
    }

    #[test]
    fn add_operator_wiring_filter_accepts_matching_first_input() {
        let registry = otd_engine::registry();
        let blur = registry.get("blurTOP").unwrap();
        assert!(op_wires_after(blur, otd_core::Family::Top));
        assert!(!op_wires_after(blur, otd_core::Family::Chop));
    }

    /// Filling Feedback's target must not make it a cook dependency.
    ///
    /// A `path_ref` parameter becomes an edge in the cook graph. On a Feedback
    /// TOP that edge is a cycle — the whole reason it reads the *previous*
    /// frame — so this pins the flag off rather than leaving the next person
    /// to "tidy up" the inconsistency and break every feedback loop at once.
    #[test]
    fn feedbacks_target_is_not_a_cook_dependency() {
        let registry = otd_engine::registry();
        let def = registry.get(otd_gpu::ops::FEEDBACK).unwrap();
        let params = (def.params)();
        assert!(
            !params["target"].is_path_ref(),
            "feedbackTOP.target became a path ref, which makes the loop a cycle"
        );
    }
