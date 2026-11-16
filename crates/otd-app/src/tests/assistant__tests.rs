use super::*;

use super::*;

    #[test]
    fn resetting_context_discards_an_inflight_reply() {
        let mut assistant = Assistant::default();
        let (tx, rx) = std::sync::mpsc::channel();
        assistant.pending = Some(rx);
        assistant.submitted_prompt = "old project".into();
        assistant.reset_context();
        assert!(!assistant.busy());
        assert!(assistant.submitted_prompt.is_empty());
        assert!(tx.send(Ok(("old plan".into(), None))).is_err());
    }

    #[test]
    fn it_opens_on_a_provider_that_is_usable() {
        // The default has to be useful on a machine where exactly one of the
        // five is configured, which is the normal case — and "configured"
        // means a key for three of them and an installed CLI for two.
        let assistant = Assistant::default();
        if Provider::ALL.iter().any(|p| assistant.ready(*p)) {
            assert!(
                assistant.ready(assistant.provider),
                "opened on {:?}, which is not usable",
                assistant.provider
            );
        }
        assert!(!assistant.busy());
        // Codex's default model is deliberately empty, so `model` may be too;
        // what must hold is that it is one this provider offers.
        assert!(
            assistant
                .provider
                .models()
                .contains(&assistant.model.as_str()),
            "{:?} opened on a model it does not list: {:?}",
            assistant.provider,
            assistant.model
        );
    }

    #[test]
    fn a_cli_provider_never_asks_for_a_key() {
        // The bar says "not installed" rather than "no key", and no amount of
        // typing in a key field it does not have makes it configured.
        let assistant = Assistant {
            provider: Provider::ClaudeCode,
            key_input: "sk-ant-whatever".into(),
            ..Default::default()
        };
        assert_eq!(
            assistant.not_configured(),
            !assistant.ready(Provider::ClaudeCode)
        );
        assert!(Provider::ClaudeCode.env_var().is_none());
    }

    #[test]
    fn a_picture_on_its_own_is_a_complete_request() {
        // The send button is gated on this, and an attached reference with an
        // empty box has to enable it — "build this" needs no words.
        let mut assistant = Assistant::default();
        assert!(!assistant.has_request(), "nothing typed, nothing attached");
        assistant.prompt = "  ".into();
        assert!(!assistant.has_request(), "whitespace is not a request");
        assistant.prompt = "a blue tunnel".into();
        assert!(assistant.has_request());
    }

    #[test]
    fn only_images_are_taken_as_references() {
        // Everything else dropped on the bar belongs to the media router, and
        // eating it silently would lose the drop entirely.
        for name in ["reference.png", "shot.JPG", "grab.webp"] {
            assert!(looks_like_an_image(std::path::Path::new(name)), "{name}");
        }
        for name in ["clip.mov", "patch.otd", "shader.glsl", "notes", "a.png.txt"] {
            assert!(!looks_like_an_image(std::path::Path::new(name)), "{name}");
        }
    }

    #[test]
    fn a_long_filename_is_shortened_from_the_middle() {
        // Both ends carry meaning — the subject at the front, the extension at
        // the back — so it is the middle that goes.
        let short = short_name("ref.png");
        assert_eq!(short, "ref.png");
        let long = short_name("a-very-long-reference-screenshot-name.png");
        assert!(long.chars().count() <= 18, "{long}");
        assert!(long.starts_with("a-very-lon"), "{long}");
        assert!(long.ends_with(".png"), "{long}");
    }

    #[test]
    fn an_empty_model_reads_as_the_clis_own_default() {
        assert_eq!(model_label(""), "(CLI default)");
        assert_eq!(model_label("  "), "(CLI default)");
        assert_eq!(model_label("sonnet"), "sonnet");
    }

    #[test]
    fn the_suggestions_are_prompts_rather_than_labels() {
        // They are pasted into the box verbatim, so each has to read as a
        // request on its own.
        for s in suggestions() {
            assert!(s.len() > 20, "{s}");
            assert!(!s.ends_with('.'), "{s}");
        }
    }
