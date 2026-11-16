use super::*;

use super::*;

    #[test]
    fn light_theme_lightens_classic_network() {
        let dark = network(UiPalette::Classic);
        let light = network_for_appearance(UiPalette::Classic, false);
        assert!(light.canvas_bg.r() > dark.canvas_bg.r());
        assert!(light.node_text.r() < 120);
    }

    #[test]
    fn system_prefers_os_theme_over_stale_ctx() {
        assert!(effective_dark(
            UiTheme::System,
            UiPalette::Classic,
            Some(Theme::Dark),
            Theme::Light,
        ));
        assert!(!effective_dark(
            UiTheme::System,
            UiPalette::Classic,
            Some(Theme::Light),
            Theme::Dark,
        ));
    }
