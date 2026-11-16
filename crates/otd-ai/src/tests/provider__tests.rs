use super::*;

use super::*;

    #[test]
    fn anthropic_and_openai_bodies_differ_where_they_have_to() {
        let anthropic = Request::new(Provider::Anthropic, "claude-sonnet-4-5-20250929")
            .system("be terse")
            .user("hello");
        let body = anthropic.body();
        // Anthropic takes the system prompt as a field, not a message.
        assert_eq!(body["system"], "be terse");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["max_tokens"], 8192);

        let openai = Request::new(Provider::OpenAi, "gpt-5")
            .system("be terse")
            .user("hello");
        let body = openai.body();
        assert!(body.get("system").is_none());
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["content"], "hello");
    }

    #[test]
    fn openrouter_is_openai_on_the_wire() {
        let a = Request::new(Provider::OpenRouter, "x")
            .system("s")
            .user("u");
        let b = Request::new(Provider::OpenAi, "x").system("s").user("u");
        assert_eq!(a.body(), b.body());
        assert_ne!(a.provider.endpoint(), b.provider.endpoint());
    }

    #[test]
    fn each_shape_of_reply_is_understood() {
        let anthropic = serde_json::json!({
            "content": [
                { "type": "thinking", "thinking": "hmm" },
                { "type": "text", "text": "the answer" }
            ],
            "stop_reason": "end_turn"
        });
        assert_eq!(
            extract_text(Provider::Anthropic, &anthropic).unwrap(),
            "the answer"
        );

        let openai = serde_json::json!({
            "choices": [{ "message": { "content": "the answer" }, "finish_reason": "stop" }]
        });
        assert_eq!(
            extract_text(Provider::OpenAi, &openai).unwrap(),
            "the answer"
        );
        assert_eq!(
            extract_text(Provider::OpenRouter, &openai).unwrap(),
            "the answer"
        );
    }

    #[test]
    fn a_two_hundred_carrying_an_error_is_still_an_error() {
        // Both providers do this, and treating it as an empty reply would
        // report "the model returned nothing" for an expired key.
        let body = serde_json::json!({ "error": { "message": "insufficient_quota" } });
        let e = extract_text(Provider::OpenAi, &body).unwrap_err();
        assert!(e.contains("insufficient_quota"), "{e}");
    }

    #[test]
    fn an_empty_reply_says_why_it_was_empty() {
        let body = serde_json::json!({
            "choices": [{ "message": { "content": "" }, "finish_reason": "length" }]
        });
        let e = extract_text(Provider::OpenAi, &body).unwrap_err();
        assert!(e.contains("length"), "{e}");
    }

    #[test]
    fn a_failure_reports_what_the_provider_said_not_just_the_number() {
        // "invalid x-api-key" and "your credit balance is too low" are
        // different problems that both arrive as 401-shaped disappointment.
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let text = explain_status(401, body);
        assert!(text.contains("invalid x-api-key"), "{text}");
        assert!(text.contains("check the key"), "{text}");

        let openai = r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota"}}"#;
        assert!(explain_status(429, openai).contains("exceeded your current quota"));

        // A body that is not JSON at all — an HTML error page from a proxy —
        // still has to produce something a person can act on.
        let html = explain_status(503, "<html>Service Unavailable</html>");
        assert!(html.contains("503"), "{html}");
        assert!(html.contains("try again"), "{html}");

        assert!(explain_status(404, "").contains("model"));
    }

    #[test]
    fn provider_names_round_trip() {
        for p in Provider::ALL {
            assert_eq!(Provider::parse(p.id()), Some(*p));
            assert_eq!(Provider::parse(&p.id().to_uppercase()), Some(*p));
            assert!(!p.models().is_empty());
            assert!(p.console_url().starts_with("https://"));
        }
        assert_eq!(Provider::parse("gemini"), None);
    }

    #[test]
    fn a_key_provider_has_somewhere_to_send_the_key() {
        // The two halves of the enum, stated as the invariant that keeps
        // `complete` honest: needing a key and having an endpoint are the
        // same question asked twice, and nothing may answer them differently.
        for p in Provider::ALL {
            assert_eq!(
                p.needs_key(),
                p.endpoint().is_some(),
                "{p:?} disagrees with itself"
            );
            assert_eq!(p.needs_key(), p.env_var().is_some(), "{p:?}");
            if p.needs_key() {
                assert!(p.endpoint().unwrap().starts_with("https://"));
                assert!(!p.default_model().is_empty(), "{p:?}");
            }
        }
        assert!(!Provider::ClaudeCode.needs_key());
        assert!(!Provider::Codex.needs_key());
    }

    #[test]
    fn codex_defaults_to_whatever_its_cli_is_configured_for() {
        // Empty is deliberate and `cli::complete` reads it as "pass no
        // -m flag". Codex hard-fails on a model the account cannot use, and
        // the plan decides which those are, so there is no safe guess to
        // make from here.
        assert_eq!(Provider::Codex.default_model(), "");
        // Claude Code has no such problem: the aliases always resolve.
        assert_eq!(Provider::ClaudeCode.default_model(), "sonnet");
    }

    /// A tiny real PNG, so the image path is exercised with bytes rather than
    /// a stub that cannot be decoded.
    fn image() -> crate::vision::Image {
        let buffer = image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(buffer)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        crate::vision::Image::decode(&png.into_inner()).unwrap()
    }

    #[test]
    fn without_an_image_the_user_turn_stays_a_plain_string() {
        // The shape three providers have always taken, and the one thing that
        // must not change for anybody not using this feature.
        let request = Request::new(Provider::Anthropic, "m").user("hello");
        assert_eq!(request.body()["messages"][0]["content"], "hello");
        let openai = Request::new(Provider::OpenAi, "m").user("hello");
        assert_eq!(openai.body()["messages"][1]["content"], "hello");
    }

    #[test]
    fn an_image_goes_out_in_each_providers_own_dialect() {
        let request = Request::new(Provider::Anthropic, "m")
            .user("rebuild this")
            .image(image());
        let content = &request.body()["messages"][0]["content"];
        // Image first: the instructions after it are about the picture.
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["type"], "base64");
        assert_eq!(content[0]["source"]["media_type"], "image/jpeg");
        assert!(!content[0]["source"]["data"].as_str().unwrap().is_empty());
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[1]["text"], "rebuild this");

        // OpenAI takes the same picture as a data URI, and OpenRouter is
        // OpenAI on the wire here too.
        for provider in [Provider::OpenAi, Provider::OpenRouter] {
            let request = Request::new(provider, "m")
                .user("rebuild this")
                .image(image());
            let content = &request.body()["messages"][1]["content"];
            assert_eq!(content[0]["type"], "image_url");
            let url = content[0]["image_url"]["url"].as_str().unwrap();
            assert!(url.starts_with("data:image/jpeg;base64,"), "{url:.40}");
            assert_eq!(content[1]["text"], "rebuild this");
        }
    }

    #[test]
    fn a_cli_provider_has_no_wire_format() {
        // Rather than quietly borrowing OpenAI's, which would produce a body
        // that looks sendable and is not.
        let request = Request::new(Provider::ClaudeCode, "sonnet")
            .system("s")
            .user("u");
        assert!(request.body().is_null());
        let body = serde_json::json!({ "choices": [] });
        assert!(extract_text(Provider::Codex, &body).is_err());
    }
