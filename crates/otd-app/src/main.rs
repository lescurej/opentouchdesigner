//! OpenTouchDesigner — the editor shell.

mod app;
mod assistant;
mod canvas;
mod code_editor;
mod code_lint;
mod container_view;
mod create_dialog;
mod dat_external;
mod engine_ui;
mod inspector;
mod macos_open;
mod media;
mod monitors;
mod params;
mod settings;
mod shortcut_routing;
mod td_import;
mod textport;
mod ui_palette;
mod watch;
mod window_comp;
mod window_layout;
mod window_placement;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 980.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("OpenTouchDesigner"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    let project = std::env::args().nth(1).map(std::path::PathBuf::from);

    eframe::run_native(
        "OpenTouchDesigner",
        options,
        Box::new(move |cc| {
            let mut app = app::OtdApp::new(cc)?;
            app.document.pending_launch = if let Some(path) = project {
                app::PendingLaunch::Open(path)
            } else {
                app::PendingLaunch::Startup
            };
            Ok(Box::new(app))
        }),
    )
}