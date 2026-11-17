use super::*;

    #[test]
    fn select_pattern_matches_tail_after_star_slash() {
        assert!(pattern_matches_operator("*/dmx", "dmx", "/comp/dmx"));
        assert!(!pattern_matches_operator("*/dmx", "dmxin1", "/comp/dmxin1"));
        assert!(!pattern_matches_operator("*/dmx", "other", "/comp/other"));
        assert!(!pattern_matches_operator("*", "dmx", "/comp/dmx"));
        assert!(pattern_matches_operator("*/dmx*", "dmxin1", "/comp/dmxin1"));
    }

    #[test]
    fn count_from_closed_range() {
        assert_eq!(channel_count_from_name("chan[1-4]"), 4);
        assert_eq!(channel_count_from_name("chan[1-16]"), 16);
        assert_eq!(channel_count_from_name("wave[10]"), 1);
        assert_eq!(channel_count_from_name("noise"), 1);
        assert_eq!(channel_count_from_name("chan[1-]"), 1);
        assert_eq!(channel_count_from_name("t[xyz]"), 3);
    }

    #[test]
    fn component_pattern_expands_translate_channels() {
        assert_eq!(expand_channel_names("t[xyz]", 3), vec!["tx", "ty", "tz"]);
        assert_eq!(expand_channel_names("r[xyz]", 3), vec!["rx", "ry", "rz"]);
        assert_eq!(expand_channel_names("s[xy]", 2), vec!["sx", "sy"]);
        assert_eq!(expand_channel_name_field("t[xyz]"), vec!["tx", "ty", "tz"]);
    }

    #[test]
    fn expand_field_whitespace_and_range() {
        assert_eq!(
            expand_channel_name_field("chan[1-3]"),
            vec!["chan1", "chan2", "chan3"]
        );
        assert_eq!(
            expand_channel_name_field("tx ty tz"),
            vec!["tx", "ty", "tz"]
        );
        assert_eq!(
            expand_channel_name_field("chan[1-2] tail"),
            vec!["chan1", "chan2", "tail"]
        );
        assert_eq!(
            expand_channel_name_field("t[xyz] bass"),
            vec!["tx", "ty", "tz", "bass"]
        );
    }

    #[test]
    fn expands_closed_range() {
        assert_eq!(
            expand_channel_names("chan[1-4]", 4),
            vec!["chan1", "chan2", "chan3", "chan4"]
        );
    }

    #[test]
    fn expands_past_range_end_for_extra_channels() {
        assert_eq!(
            expand_channel_names("chan[1-3]", 5),
            vec!["chan1", "chan2", "chan3", "chan4", "chan5"]
        );
    }

    #[test]
    fn select_pattern_matches_range() {
        assert!(pattern_matches_channel("chan[1-3]", "chan2"));
        assert!(!pattern_matches_channel("chan[1-3]", "chan4"));
        assert!(pattern_matches_channel("tx*", "tx1"));
        assert!(pattern_matches_channel("t[xyz]", "ty"));
        assert!(!pattern_matches_channel("t[xyz]", "t1"));
    }

    #[test]
    fn space_separates_channel_names() {
        assert_eq!(channel_count_from_name("chan1 chan2"), 2);
        assert_eq!(
            expand_channel_names("chan1 chan2", 2),
            vec!["chan1", "chan2"]
        );
        assert_eq!(
            expand_channel_names("chan[1-2] bass", 3),
            vec!["chan1", "chan2", "bass"]
        );
    }

    #[test]
    fn rename_to_at_numbered() {
        assert_eq!(rename_to_at("out[1-]", 0, "x"), "out1");
        assert_eq!(rename_to_at("out[1-]", 2, "x"), "out3");
        assert_eq!(rename_to_at("a b", 1, "x"), "b");
        assert_eq!(rename_to_at("", 0, "keep"), "keep");
        assert_eq!(rename_to_at("t[xyz]", 0, "x"), "tx");
        assert_eq!(rename_to_at("t[xyz]", 2, "x"), "tz");
        assert_eq!(rename_to_at("t[xyz]", 9, "keep"), "keep");
    }

    #[test]
    fn parse_numbered_and_component_templates() {
        assert_eq!(
            parse_numbered_pattern("out[10-20]"),
            Some(NumberedPattern {
                prefix: "out",
                start: 10,
                end: Some(20),
                suffix: "",
            })
        );
        assert_eq!(
            parse_component_pattern("t[xyz]"),
            Some(ComponentPattern {
                prefix: "t",
                components: "xyz",
                suffix: "",
            })
        );
        assert!(parse_numbered_pattern("t[xyz]").is_none());
        assert!(parse_component_pattern("chan[1-3]").is_none());
        assert!(parse_name_pattern("no_brackets").is_none());
        assert!(parse_name_pattern("empty[]").is_none());
    }

    #[test]
    fn hyphen_in_brackets_prefers_numeric_range() {
        assert_eq!(channel_count_from_name("n[a-b]"), 1);
        assert_eq!(expand_channel_names("n[a-b]", 1), vec!["n1"]);
    }

    #[test]
    fn reversed_numeric_range_clamps_end_to_start() {
        assert_eq!(channel_count_from_name("chan[9-4]"), 1);
        assert!(numbered_pattern_matches(
            &NumberedPattern {
                prefix: "chan",
                start: 9,
                end: Some(9),
                suffix: "",
            },
            "chan9"
        ));
        assert!(!numbered_pattern_matches(
            &NumberedPattern {
                prefix: "chan",
                start: 9,
                end: Some(9),
                suffix: "",
            },
            "chan10"
        ));
    }

    #[test]
    fn open_ended_numbered_range() {
        let open = parse_numbered_pattern("out[1-]").unwrap();
        assert_eq!(numbered_channel_count(&open), 1);
        assert!(numbered_pattern_matches(&open, "out99"));
        assert!(!numbered_pattern_matches(&open, "out0"));
        assert_eq!(channel_name_at(&open, 0), "out1");
        assert_eq!(channel_name_at(&open, 4), "out5");
    }

    #[test]
    fn numbered_and_component_match_with_affixes() {
        let num = NumberedPattern {
            prefix: "L",
            start: 1,
            end: Some(2),
            suffix: "_v",
        };
        assert!(numbered_pattern_matches(&num, "L1_v"));
        assert!(!numbered_pattern_matches(&num, "L1"));
        assert!(!numbered_pattern_matches(&num, "L3_v"));
        let comp = ComponentPattern {
            prefix: "t",
            components: "xyz",
            suffix: "w",
        };
        assert!(component_pattern_matches(&comp, "txw"));
        assert!(!component_pattern_matches(&comp, "tx"));
        assert!(!component_pattern_matches(&comp, "txyz"));
    }

    #[test]
    fn glob_and_multi_token_channel_patterns() {
        assert!(glob_match("pre*post", "pre123post"));
        assert!(!glob_match("pre*post", "prepostextra"));
        assert!(pattern_matches_channel("chan1 chan2", "chan2"));
        assert!(!pattern_matches_channel("chan1 chan2", "chan3"));
    }

    #[test]
    fn caret_excludes_one_or_many_channels() {
        assert!(!pattern_matches_channel("^chan1", "chan1"));
        assert!(pattern_matches_channel("^chan1", "chan2"));
        assert!(!pattern_matches_channel("* ^chan1 ^chan3", "chan1"));
        assert!(!pattern_matches_channel("* ^chan1 ^chan3", "chan3"));
        assert!(pattern_matches_channel("* ^chan1 ^chan3", "chan2"));
        assert!(!pattern_matches_channel("^chan1 ^chan3", "chan1"));
        assert!(!pattern_matches_channel("^chan1 ^chan3", "chan3"));
        assert!(pattern_matches_channel("^chan1 ^chan3", "chan2"));
        assert!(pattern_matches_channel("chan* ^chan1 ^chan2", "chan3"));
        assert!(!pattern_matches_channel("chan* ^chan1 ^chan2", "chan1"));
        assert!(!pattern_matches_channel("chan* ^chan1 ^chan2", "other"));
        assert!(!pattern_matches_channel("chan[1-4] ^chan[2-3]", "chan2"));
        assert!(!pattern_matches_channel("chan[1-4] ^chan[2-3]", "chan3"));
        assert!(pattern_matches_channel("chan[1-4] ^chan[2-3]", "chan1"));
        assert!(pattern_matches_channel("chan[1-4] ^chan[2-3]", "chan4"));
        assert!(!pattern_matches_channel("^t[xy]", "tx"));
        assert!(!pattern_matches_channel("^t[xy]", "ty"));
        assert!(pattern_matches_channel("^t[xy]", "tz"));
        assert!(!pattern_matches_channel("^chan*", "chan9"));
        assert!(pattern_matches_channel("^chan*", "other"));
        assert!(pattern_matches_channel("chan?", "chan1"));
        assert!(!pattern_matches_channel("chan?", "chan"));
        assert!(!pattern_matches_channel("chan?", "chan12"));
        assert!(!pattern_matches_channel("^?/heartbeat", "a/heartbeat"));
        assert!(!pattern_matches_channel("^?/heartbeat", "1/heartbeat"));
        assert!(pattern_matches_channel("^?/heartbeat", "ab/heartbeat"));
        assert!(pattern_matches_channel("^?/heartbeat", "node/heartbeat"));
        assert!(!pattern_matches_channel("^*/heartbeat", "node/heartbeat"));
        assert!(!pattern_matches_channel("^*/heartbeat", "a/b/heartbeat"));
        assert!(pattern_matches_channel("^*/heartbeat", "heartbeat"));
        assert!(!pattern_matches_channel("^?/heartbeat ^campbel/batt", "a/heartbeat"));
        assert!(!pattern_matches_channel("^?/heartbeat ^campbel/batt", "1/heartbeat"));
        assert!(!pattern_matches_channel("^?/heartbeat ^campbel/batt", "campbel/batt"));
        assert!(pattern_matches_channel("^?/heartbeat ^campbel/batt", "ab/heartbeat"));
        assert!(pattern_matches_channel("^?/heartbeat ^campbel/batt", "keep"));
        assert!(!pattern_matches_channel("^campbel/batt", "campbel/batt"));
        assert!(pattern_matches_channel("campbel/batt", "campbel/batt"));
    }

    #[test]
    fn expand_non_pattern_token_appends_indices() {
        assert_eq!(
            expand_channel_names("tail", 3),
            vec!["tail1", "tail2", "tail3"]
        );
        assert!(expand_channel_names("", 3).is_empty());
        assert_eq!(expand_channel_name_field(""), vec!["chan1"]);
    }

    #[test]
    fn channel_seed_is_stable_and_index_sensitive() {
        let a = channel_seed_for_name(42, "chan1", 0);
        let b = channel_seed_for_name(42, "chan1", 0);
        let c = channel_seed_for_name(42, "chan1", 1);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, channel_seed_for_name(43, "chan1", 0));
    }

    #[test]
    fn operator_reference_is_pattern_flags() {
        assert!(operator_reference_is_pattern("foo*"));
        assert!(operator_reference_is_pattern("x?y"));
        assert!(operator_reference_is_pattern("chan[1-3]"));
        assert!(operator_reference_is_pattern("^comp"));
        assert!(!operator_reference_is_pattern("plain_name"));
    }

    #[test]
    fn pattern_matches_operator_path_and_empty() {
        assert!(!pattern_matches_operator("", "dmx", "/comp/dmx"));
        assert!(pattern_matches_operator("comp/dmx", "other", "/comp/dmx"));
        assert!(pattern_matches_operator("chan[1-2]", "chan1", "/x"));
        assert!(!pattern_matches_operator("^comp", "other", "/x/other"));
        assert!(!pattern_matches_operator("^comp", "comp", "/x/comp"));
        assert!(pattern_matches_operator("dmx ^other", "dmx", "/x/dmx"));
        assert!(!pattern_matches_operator("dmx ^dmx", "dmx", "/x/dmx"));
    }
