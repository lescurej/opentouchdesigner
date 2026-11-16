use super::*;

use super::*;

    #[test]
    fn removals_are_dropped_unless_the_user_asked_for_them() {
        let mut plan = otd_ai::Plan {
            deletes: vec!["blur1".into(), "over1".into()],
            ..Default::default()
        };
        let warning = gate_deletes(&mut plan, false).expect("a spared delete is worth saying");
        assert!(warning.contains("blur1") && warning.contains("over1"));
        assert!(plan.deletes.is_empty(), "removal is off: nothing goes");

        let mut plan = otd_ai::Plan {
            deletes: vec!["blur1".into()],
            ..Default::default()
        };
        assert_eq!(gate_deletes(&mut plan, true), None);
        assert_eq!(plan.deletes, vec!["blur1".to_string()]);
    }

    #[test]
    fn a_model_name_is_shortened_to_something_that_fits_a_chip() {
        // The provider prefix and the date suffix are the two parts nobody
        // reads, and together they are most of the string.
        assert_eq!(
            short_model("anthropic/claude-sonnet-4.5"),
            "claude-sonnet-4.5"
        );
        assert_eq!(
            short_model("claude-sonnet-4-5-20250929"),
            "claude-sonnet-4-5"
        );
        assert_eq!(short_model("gpt-5"), "gpt-5");
        // A date-like tail that is not a date is left alone.
        assert_eq!(short_model("llama-4-maverick"), "llama-4-maverick");
        // And anything still absurd is truncated rather than blowing the row.
        assert!(short_model(&"x".repeat(60)).chars().count() <= 22);
    }

    #[test]
    fn a_status_line_is_one_line() {
        let notes = "Built a tunnel.\nTurn decay1.brightness for trail length.";
        assert_eq!(one_line(notes), "Built a tunnel");
        assert!(!one_line(notes).contains('\n'));
        let long = "a ".repeat(200);
        assert!(one_line(&long).chars().count() <= 90);
    }

    #[test]
    fn the_bar_is_closed_until_asked_for() {
        let mut a = Assistant::default();
        assert!(!a.bar);
        assert!(!a.collapsed);
        a.bar = true;
        a.collapsed = true;
        assert!(a.bar, "collapsing is not hiding");
        a.bar = false;
        assert!(!a.bar);
    }
