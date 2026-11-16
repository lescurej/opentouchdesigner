use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;
use otd_core::{CookContext, History, NodeClipboard, Project};
use otd_engine::ProjectRuntime;
use otd_gpu::GpuContext;

use crate::app::file_dialog;
use crate::engine_ui::UiEngine;

use super::super::*;

impl OtdApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        let render_state = cc
            .wgpu_render_state
            .clone()
            .ok_or("OpenTouchDesigner needs the wgpu backend")?;

        let registry = otd_engine::registry();
        let settings = crate::settings::AppSettings::load();
        let graph = otd_core::Graph::new();
        let time = CookContext::default();
        let gpu = GpuContext::new(render_state.device.clone(), render_state.queue.clone());
        let engine = UiEngine::new(graph.clone(), registry.clone(), time, gpu);
        let root = graph.root();

        let mut app = OtdApp {
            graph,
            registry,
            engine,
            last_frame_timing: otd_engine::FrameTiming::default(),
            render_state,
            time,
            transport: TransportState {
                playing: true,
                smoothed_frame_ms: 1000.0 / 60.0,
                smoothed_wall_tick_ms: 1000.0 / 60.0,
                ..TransportState::default()
            },
            canvas: CanvasState {
                selected: None,
                selection: std::collections::BTreeSet::new(),
                selected_wire: None,
                wire_menu_at: None,
                viewer: None,
                current: root,
                view: crate::canvas::CanvasView::default(),
                network_views: std::collections::HashMap::new(),
                canvas_rect: egui::Rect::NOTHING,
                drag: None,
                node_drag_start: None,
                pending_param_drop: None,
                open_container_views: std::collections::BTreeSet::new(),
                callback_embedded: std::collections::BTreeSet::new(),
                viewer_top_pan: std::collections::HashMap::new(),
                viewer_top_zoom: std::collections::HashMap::new(),
                viewer_dat_scroll: std::collections::HashMap::new(),
                visible: Vec::new(),
                skip_canvas_deselect_once: false,
                canvas_rename: None,
                network_view_id: None,
            },
            perform_shell: PerformShell {
                perform: false,
                perform_edit: false,
                perform_applied: false,
                recorder: None,
                output_window: false,
                output_fullscreen: false,
                show_window_placement: false,
                main_perform_window: None,
                main_perform_window_path: String::new(),
                main_perform_ui_path: String::new(),
                start_in_perform_mode: false,
                designer_custom_position: false,
                designer_x: 100.0,
                designer_y: 100.0,
                designer_w: 1600.0,
                designer_h: 980.0,
                designer_position_pending: false,
                output_closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                window_close_flags: std::collections::HashMap::new(),
                window_open_anchors: std::collections::HashMap::new(),
                window_was_open: std::collections::HashMap::new(),
            },
            document: DocumentState {
                history: History::new(settings.undo_limit),
                project_path: None,
                status: String::new(),
                cook_error: None,
                channel_expr_clipboard: String::new(),
                node_clipboard: NodeClipboard::default(),
                paste_count: 0,
                document_dirty: false,
                show_close_save_prompt: false,
                pending_launch: super::PendingLaunch::None,
            },
            graph_commands: Vec::new(),
            pending_media_drop: None,
            create_dialog: None,
            create_dialog_rect: None,
            td_import: None,
            td_import_rect: None,
            show_params_panel: true,
            show_perf: true,
            show_monitor: false,
            show_textport: false,
            textport: crate::textport::Textport::default(),
            code_lint: crate::code_lint::CodeLintCache::default(),
            input_state: otd_chop::InputState::default(),
            new_param_name: String::new(),
            new_param_type: "float".to_string(),
            param_inspector_page: std::collections::HashMap::new(),
            thumbs: std::collections::HashMap::new(),
            thumb_sizes: std::collections::HashMap::new(),
            pending_texture_sync: Vec::new(),
            ui_snapshot: super::engine_snapshot::EngineUiSnapshot::default(),
            assistant: crate::assistant::Assistant::default(),
            settings,
            show_settings: false,
            settings_draft: None,
            floating_ui_rects: Vec::new(),
            params_panel_rect: egui::Rect::NOTHING,
            external_dat_edits: std::collections::HashMap::new(),
            file_dialogs: super::file_dialog::FileDialogState::default(),
            monitors: Vec::new(),
            blur_params_on_select: false,
            background_engine: super::minimized_cook::BackgroundEngine::new(),
            playback_activity: super::playback_activity::PlaybackActivity::new(),
            frame_perf_probe: super::FramePerfProbe::default(),
            last_logic_at: std::time::Instant::now(),
        };

        app.apply_settings(&cc.egui_ctx);
        app.textport
            .info(0, "Textport — cook errors, callbacks, and Python (Enter to run).");
        if let Some(e) = app.engine.python_error() {
            app.textport.error(0, e);
        }
        Ok(app)
    }

    pub(crate) fn runtime(&self) -> &Arc<Mutex<ProjectRuntime>> {
        self.engine.runtime()
    }

    pub(crate) fn engine_rt(&self) -> std::sync::MutexGuard<'_, ProjectRuntime> {
        self.engine
            .runtime
            .lock()
            .expect("engine runtime lock")
    }

    pub(crate) fn effective_settings(&self) -> &crate::settings::AppSettings {
        if self.show_settings {
            if let Some(draft) = &self.settings_draft {
                return draft;
            }
        }
        &self.settings
    }

    pub(crate) fn mark_view_dirty(&mut self) {
        self.document.document_dirty = true;
    }

    pub fn apply_settings(&mut self, ctx: &egui::Context) {
        self.show_perf = self.settings.show_node_cook_time;
        self.document.history.set_limit(self.settings.undo_limit);
        crate::ui_palette::apply(
            ctx,
            self.settings.ui_palette,
            self.settings.ui_theme,
            None,
        );
    }

    pub(crate) fn ask_save_project(&mut self, then_close: bool) {
        let mut dialog = rfd::AsyncFileDialog::new()
            .add_filter("OpenTouchDesigner project", &["otd"])
            .set_file_name("untitled.otd");
        if let Some(dir) = self.document.project_path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        file_dialog::request(
            self,
            dialog,
            file_dialog::Kind::SaveFile,
            file_dialog::Action::SaveProject { then_close },
        );
    }

    pub fn write_project_file(&mut self, path: &Path) {
        let mut project = Project::from_graph(&self.graph, &self.registry, self.time.fps);
        project.sample_rate = otd_core::canonical_project_sample_rate(self.time.sample_rate);
        project.settings = self.project_settings();
        match project.save(path) {
            Ok(()) => {
                self.document.status = format!("Saved {}", path.display());
                self.document.project_path = Some(path.to_path_buf());
                self.document.document_dirty = false;
                self.settings.last_opened_path = path.to_path_buf();
                let _ = self.settings.save();
            }
            Err(e) => self.document.status = format!("Save failed: {e}"),
        }
    }

    pub fn load_project_file(&mut self, path: PathBuf) {
        match Project::load(&path) {
            Ok(project) => match project.to_graph(&self.registry) {
                Ok(graph) => {
                    self.assistant.reset_context();
                    self.graph = graph;
                    self.engine.reset_cook_state();
                    for id in self.thumbs.keys().copied().collect::<Vec<_>>() {
                        self.free_thumbnail(id);
                    }
                    self.thumbs.clear();
                    self.thumb_sizes.clear();
                    self.document.history.clear();
                    self.clear_selection();
                    self.canvas.viewer = self
                        .graph
                        .walk()
                        .into_iter()
                        .rev()
                        .find(|&id| self.graph.node(id).family == otd_core::Family::Comp);
                    self.apply_project_settings(&project.settings);
                    self.time.sample_rate =
                        otd_core::canonical_project_sample_rate(project.sample_rate);
                    otd_engine::panel_templates::upgrade_panel_internals(
                        &mut self.graph,
                        &self.registry,
                    );
                    {
                        let mut rt = self.engine_rt();
                        rt.graph.clone_from(&self.graph);
                        rt.time.clone_from(&self.time);
                    }
                    self.canvas.current = self.graph.root();
                    self.canvas.view = crate::canvas::CanvasView::default();
                    self.canvas.network_views.clear();
                    self.document.document_dirty = false;
                    self.document.status = format!("Opened {}", path.display());
                    self.document.project_path = Some(path.clone());
                    self.settings.last_opened_path = path;
                    let _ = self.settings.save();
                }
                Err(e) => self.document.status = format!("Open failed: {e}"),
            },
            Err(e) => self.document.status = format!("Open failed: {e}"),
        }
    }

    pub fn write_component_file(&mut self, id: otd_core::NodeId, path: &Path) {
        if !self.graph.contains(id) {
            return;
        }
        match otd_core::Component::from_graph(&self.graph, id, &self.registry) {
            Some(c) => match c.save(path) {
                Ok(()) => self.document.status = format!("Saved component {}", path.display()),
                Err(e) => self.document.status = format!("Save failed: {e}"),
            },
            None => self.document.status = "could not read that component".into(),
        }
    }

    pub fn export_bundle_into(&mut self, dir: &Path, name: &str) {
        match otd_core::bundle::export(
            &self.graph,
            &self.registry,
            self.time.fps,
            self.time.sample_rate,
            dir,
            name,
        ) {
            Ok(bundle) => {
                self.document.status = format!(
                    "Exported {} — {} component(s), {} media file(s)",
                    bundle.project.display(),
                    bundle.components.len(),
                    bundle.media.len()
                );
            }
            Err(e) => self.document.status = format!("Export failed: {e}"),
        }
    }

}