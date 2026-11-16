use super::*;

    fn any_font() -> Option<&'static Font> {
        default_font()
    }

    #[test]
    fn text_puts_ink_on_the_canvas() {
        let Some(font) = any_font() else {
            eprintln!("skipping: no system font on this machine");
            return;
        };
        let raster = rasterise(
            font,
            &Layout {
                text: "Hi".into(),
                size: 48.0,
                line_spacing: 1.2,
                horizontal: Align::Centre,
                vertical: Align::Centre,
                width: 200,
                height: 100,
                wrap: false,
            },
        );
        let ink = raster.pixels.chunks(4).filter(|p| p[3] > 0).count();
        assert!(ink > 20, "expected glyphs, got {ink} covered pixels");
        // And nothing in the corner: centred text must not start at 0,0.
        assert_eq!(raster.pixels[3], 0);
    }

    #[test]
    fn empty_text_is_transparent_rather_than_black() {
        // It is the usual thing to composite over, so the default has to be
        // "nothing here", not "a black rectangle".
        let Some(font) = any_font() else { return };
        let raster = rasterise(
            font,
            &Layout {
                text: String::new(),
                size: 32.0,
                line_spacing: 1.2,
                horizontal: Align::Start,
                vertical: Align::Start,
                width: 64,
                height: 32,
                wrap: false,
            },
        );
        assert!(raster.pixels.iter().all(|b| *b == 0));
    }

    #[test]
    fn wrapping_splits_a_long_line_and_not_a_short_one() {
        let Some(font) = any_font() else { return };
        let layout = |wrap: bool, width: u32| Layout {
            text: "the quick brown fox jumps over the lazy dog".into(),
            size: 24.0,
            line_spacing: 1.2,
            horizontal: Align::Start,
            vertical: Align::Start,
            width,
            height: 400,
            wrap,
        };
        assert_eq!(lines(font, &layout(false, 100)).len(), 1);
        assert!(lines(font, &layout(true, 100)).len() > 3);
        assert_eq!(lines(font, &layout(true, 4000)).len(), 1);
    }

    #[test]
    fn a_newline_starts_a_new_line_even_without_wrapping() {
        let Some(font) = any_font() else { return };
        let got = lines(
            font,
            &Layout {
                text: "one\ntwo\nthree".into(),
                size: 24.0,
                line_spacing: 1.2,
                horizontal: Align::Start,
                vertical: Align::Start,
                width: 1000,
                height: 400,
                wrap: false,
            },
        );
        assert_eq!(got, vec!["one", "two", "three"]);
    }
