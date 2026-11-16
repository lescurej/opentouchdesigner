use super::*;

    #[test]
    fn settings_round_trip() {
        let s = AppSettings {
            startup_mode: StartupMode::LastOpened,
            startup_custom_path: PathBuf::from("/tmp/show.otd"),
            last_opened_path: PathBuf::from("/tmp/last.otd"),
            prompt_save_on_exit: false,
            undo_limit: 0,
            ui_theme: UiTheme::Light,
            ui_palette: UiPalette::Ocean,
            dat_external_editor: "open -a \"Visual Studio Code\" %path%".into(),
            network_grid_style: NetworkGridStyle::Dots,
            network_grid_opacity: 0.4,
            ..Default::default()
        };
        let back = AppSettings::parse(&s.to_text());
        assert_eq!(back, s);
    }
