use std::path::PathBuf;

use super::{theme_label, StartupMode, UiTheme};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Startup,
    Editor,
    Timeline,
    Appearance,
}

fn page_id() -> egui::Id {
    egui::Id::new("otd_settings_page")
}

pub fn show(app: &mut crate::app::OtdApp, ctx: &egui::Context) {
    if !app.show_settings {
        app.settings_draft = None;
        return;
    }
    if app.settings_draft.is_none() {
        app.settings_draft = Some(app.settings.clone());
    }
    let draft = app
        .settings_draft
        .as_mut()
        .expect("settings draft while window is open");
    let mut commit = false;
    let mut dismiss = false;
    let mut browse_startup = false;
    let mut page = ctx.data(|d| d.get_temp(page_id()).unwrap_or(Page::Startup));
    egui::Window::new("Settings")
        .open(&mut app.show_settings)
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Editor preferences — saved for this user, not in the project.")
                    .weak()
                    .small(),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut page, Page::Startup, "Startup");
                ui.selectable_value(&mut page, Page::Editor, "Editor");
                ui.selectable_value(&mut page, Page::Timeline, "Timeline");
                ui.selectable_value(&mut page, Page::Appearance, "Appearance");
            });
            ui.separator();

            match page {
                Page::Startup => startup_page(ui, draft, &mut browse_startup),
                Page::Editor => editor_page(ui, draft),
                Page::Timeline => timeline_page(ui, draft),
                Page::Appearance => appearance_page(ui, draft),
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    commit = true;
                    dismiss = true;
                }
                if ui.button("Cancel").clicked() {
                    dismiss = true;
                }
            });
        });
    ctx.data_mut(|d| d.insert_temp(page_id(), page));
    if browse_startup {
        let dialog =
            rfd::AsyncFileDialog::new().add_filter("OpenTouchDesigner project", &["otd"]);
        crate::app::file_dialog::request(
            app,
            dialog,
            crate::app::file_dialog::Kind::PickFile,
            crate::app::file_dialog::Action::SettingsStartupFile,
        );
    }
    if commit {
        if let Some(mut saved) = app.settings_draft.take() {
            saved.normalize_loop_range();
            app.settings = saved;
        }
        app.apply_settings(ctx);
        if let Err(e) = app.settings.save() {
            app.document.status = format!("Settings save failed: {e}");
        } else {
            app.document.status = "Settings saved".into();
        }
        app.show_settings = false;
    } else if dismiss || !app.show_settings {
        app.show_settings = false;
        app.settings_draft = None;
    }
}

fn startup_page(ui: &mut egui::Ui, draft: &mut super::AppSettings, browse_startup: &mut bool) {
    egui::Grid::new("otd_settings_startup")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label("Startup file mode");
            egui::ComboBox::from_id_salt("startup_mode")
                .selected_text(startup_mode_label(draft.startup_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut draft.startup_mode,
                        StartupMode::DefaultExample,
                        startup_mode_label(StartupMode::DefaultExample),
                    );
                    ui.selectable_value(
                        &mut draft.startup_mode,
                        StartupMode::Blank,
                        startup_mode_label(StartupMode::Blank),
                    );
                    ui.selectable_value(
                        &mut draft.startup_mode,
                        StartupMode::CustomFile,
                        startup_mode_label(StartupMode::CustomFile),
                    );
                });
            ui.end_row();

            if draft.startup_mode == StartupMode::DefaultExample {
                ui.label("Default example");
                egui::ComboBox::from_id_salt("startup_demo")
                    .selected_text(&draft.startup_demo)
                    .show_ui(ui, |ui| {
                        for name in otd_engine::demo::NAMES {
                            ui.selectable_value(&mut draft.startup_demo, (*name).to_string(), *name);
                        }
                    });
                ui.end_row();
            }

            if draft.startup_mode == StartupMode::CustomFile {
                ui.label("Custom startup file");
                ui.horizontal(|ui| {
                    let mut path_text = draft.startup_custom_path.display().to_string();
                    if ui.text_edit_singleline(&mut path_text).changed() {
                        draft.startup_custom_path = PathBuf::from(path_text);
                    }
                    if ui.button("Browse…").clicked() {
                        *browse_startup = true;
                    }
                });
                ui.end_row();
            }
        });
}

fn editor_page(ui: &mut egui::Ui, draft: &mut super::AppSettings) {
    ui.checkbox(&mut draft.prompt_save_on_exit, "Prompt to save on exit");
    ui.checkbox(
        &mut draft.stop_playing_when_minimized,
        "Stop playing when minimized",
    );
    ui.checkbox(&mut draft.show_node_cook_time, "Show cook time on nodes");
    ui.checkbox(
        &mut draft.blur_params_on_select,
        "Release parameter focus when selecting a node",
    );
    ui.checkbox(
        &mut draft.enable_playbar_shortcuts,
        "Enable playbar shortcuts (Space)",
    );
    ui.checkbox(&mut draft.snap_nodes_to_grid, "Snap nodes to grid");
    ui.horizontal(|ui| {
        ui.label("Undo steps (0 = off)");
        ui.add(egui::DragValue::new(&mut draft.undo_limit).speed(1).range(0..=500));
    });
}

fn timeline_page(ui: &mut egui::Ui, draft: &mut super::AppSettings) {
    ui.label(egui::RichText::new("Defaults for a new project").weak().small());
    ui.add_space(4.0);
    ui.checkbox(&mut draft.start_playing, "Start playing");
    ui.checkbox(&mut draft.default_looping, "Loop enabled");
    ui.horizontal(|ui| {
        ui.label("Loop start");
        ui.add(
            egui::DragValue::new(&mut draft.default_loop_start)
                .speed(0.1)
                .range(-86400.0..=86400.0),
        );
        ui.label("Loop end");
        ui.add(
            egui::DragValue::new(&mut draft.default_loop_end)
                .speed(0.1)
                .range(-86400.0..=86400.0),
        );
    });
    ui.horizontal(|ui| {
        ui.label("Project FPS");
        ui.add(
            egui::DragValue::new(&mut draft.default_project_fps)
                .speed(1.0)
                .range(1.0..=240.0),
        );
    });
}

fn appearance_page(ui: &mut egui::Ui, draft: &mut super::AppSettings) {
    ui.horizontal(|ui| {
        ui.label("Theme");
        egui::ComboBox::from_id_salt("ui_theme")
            .selected_text(theme_label(draft.ui_theme))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut draft.ui_theme, UiTheme::System, "System");
                ui.selectable_value(&mut draft.ui_theme, UiTheme::Dark, "Dark");
                ui.selectable_value(&mut draft.ui_theme, UiTheme::Light, "Light");
            });
    });
    ui.horizontal(|ui| {
        ui.label("Network palette");
        egui::ComboBox::from_id_salt("ui_palette")
            .selected_text(draft.ui_palette.label())
            .show_ui(ui, |ui| {
                for palette in crate::settings::UiPalette::ALL {
                    ui.selectable_value(
                        &mut draft.ui_palette,
                        palette,
                        format!("{} — {}", palette.label(), palette.summary()),
                    );
                }
            });
    });
}

pub(crate) fn startup_mode_label(mode: StartupMode) -> &'static str {
    match mode {
        StartupMode::DefaultExample => "Default example file",
        StartupMode::Blank => "Blank file",
        StartupMode::CustomFile => "Custom file",
        StartupMode::LastOpened => "Last opened file",
    }
}
