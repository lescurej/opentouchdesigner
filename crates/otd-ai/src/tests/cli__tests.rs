use super::*;

use super::*;

    #[test]
    fn a_claude_reply_is_the_result_field() {
        let out = r#"{"is_error":false,"stop_reason":"end_turn","result":"{\"ops\":[]}","type":"result"}"#;
        assert_eq!(parse_claude(out, "").unwrap(), "{\"ops\":[]}");
    }

    #[test]
    fn a_claude_failure_says_what_to_do_about_it() {
        let out =
            r#"{"is_error":true,"result":"Invalid API key · Please run /login","type":"result"}"#;
        let e = parse_claude(out, "").unwrap_err();
        assert!(e.contains("/login"), "{e}");
        assert!(e.contains("sign in"), "{e}");

        let limited = r#"{"is_error":true,"result":"5-hour limit reached","type":"result"}"#;
        let e = parse_claude(limited, "").unwrap_err();
        assert!(
            e.contains("API key provider"),
            "the way out is offered: {e}"
        );
    }

    #[test]
    fn the_result_is_found_among_the_stream_that_precedes_it() {
        // stream-json emits a system line, assistant lines and then the
        // result. Taking the first JSON object, or the last line blindly,
        // both get this wrong.
        let out = concat!(
            r#"{"type":"system","subtype":"init","session_id":"x"}"#,
            "\n",
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"partial"}]}}"#,
            "\n",
            r#"{"is_error":false,"stop_reason":"end_turn","result":"{\"ops\":[]}","type":"result"}"#,
            "\n",
        );
        assert_eq!(parse_claude(out, "").unwrap(), "{\"ops\":[]}");
    }

    #[test]
    fn a_turn_carries_an_image_as_content_blocks() {
        let buffer = image::RgbaImage::from_pixel(8, 8, image::Rgba([9, 9, 9, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(buffer)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let image = crate::vision::Image::decode(&png.into_inner()).unwrap();

        let request = Request::new(Provider::ClaudeCode, "sonnet")
            .user("rebuild this")
            .image(image);
        let line = claude_turn(&request);
        // One line, because stream-json is line-delimited and a pretty-printed
        // message would be several messages as far as the CLI is concerned.
        assert!(!line.contains('\n'), "{line:.80}");

        let parsed: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed["type"], "user");
        assert_eq!(parsed["message"]["role"], "user");
        let content = &parsed["message"]["content"];
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["media_type"], "image/jpeg");
        assert_eq!(content[1]["text"], "rebuild this");
    }

    #[test]
    fn a_turn_without_an_image_is_still_a_content_block() {
        // One envelope either way: the CLI only reads stream-json, so there is
        // no plain-text path to keep working.
        let request = Request::new(Provider::ClaudeCode, "sonnet").user("a blue tunnel");
        let parsed: serde_json::Value = serde_json::from_str(&claude_turn(&request)).unwrap();
        let content = &parsed["message"]["content"];
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "a blue tunnel");
        assert!(content[1].is_null());
    }

    #[test]
    fn a_claude_reply_that_is_not_json_is_reported_as_itself() {
        // The usual cause is the CLI printing an update notice or a crash
        // before it gets as far as producing a result object.
        let e = parse_claude("command not found: node\n", "").unwrap_err();
        assert!(e.contains("did not return JSON"), "{e}");
        assert!(e.contains("node"), "the actual text survives: {e}");
    }

    #[test]
    fn an_empty_claude_reply_says_why_it_was_empty() {
        let out = r#"{"is_error":false,"stop_reason":"max_tokens","result":"","type":"result"}"#;
        let e = parse_claude(out, "").unwrap_err();
        assert!(e.contains("max_tokens"), "{e}");
    }

    #[test]
    fn a_codex_reply_is_the_last_agent_message() {
        // Warnings arrive as completed `error` items in the same stream and
        // are not failures — this exact shape comes back on every run.
        let out = concat!(
            r#"{"type":"thread.started","thread_id":"x"}"#,
            "\n",
            r#"{"type":"item.completed","item":{"id":"i0","type":"error","message":"Skill descriptions were shortened"}}"#,
            "\n",
            r#"{"type":"item.completed","item":{"id":"i1","type":"agent_message","text":"{\"ops\":[]}"}}"#,
            "\n",
            r#"{"type":"turn.completed","usage":{"input_tokens":15638}}"#,
        );
        assert_eq!(parse_codex(out, "").unwrap(), "{\"ops\":[]}");
    }

    #[test]
    fn a_codex_failure_unwraps_the_error_it_was_handed() {
        // Codex quotes the upstream body verbatim into its own message. The
        // sentence inside is the only part worth showing.
        let out = concat!(
            r#"{"type":"turn.started"}"#,
            "\n",
            r#"{"type":"error","message":"{\"type\":\"error\",\"status\":400,\"error\":{\"type\":\"invalid_request_error\",\"message\":\"The 'gpt-5.1-codex' model is not supported when using Codex with a ChatGPT account.\"}}"}"#,
        );
        let e = parse_codex(out, "").unwrap_err();
        assert!(e.contains("not supported when using Codex"), "{e}");
        assert!(
            !e.contains("invalid_request_error"),
            "unwrapped, not quoted: {e}"
        );
    }

    #[test]
    fn a_codex_stream_with_no_reply_falls_back_to_stderr() {
        let e = parse_codex("", "codex: command failed\n").unwrap_err();
        assert!(e.contains("command failed"), "{e}");
        // And with nothing anywhere, it still says something.
        assert!(!parse_codex("", "").unwrap_err().is_empty());
    }

    #[test]
    fn only_the_cli_providers_have_a_command() {
        assert_eq!(command_name(Provider::ClaudeCode), Some("claude"));
        assert_eq!(command_name(Provider::Codex), Some("codex"));
        assert_eq!(command_name(Provider::Anthropic), None);
        assert!(binary(Provider::OpenAi).is_none());
    }

    #[test]
    fn the_tail_of_a_stream_is_the_end_of_it() {
        let text = "starting\nloading\nline one\nline two\nthe actual error";
        let tail = tail(text);
        assert!(tail.contains("the actual error"), "{tail}");
        assert!(!tail.contains("starting"), "{tail}");
        // Order is preserved: the reason usually spans the last few lines.
        assert!(tail.find("line two").unwrap() < tail.find("the actual").unwrap());
    }
