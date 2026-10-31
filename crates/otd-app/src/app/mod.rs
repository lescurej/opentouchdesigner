//! Application state and the per-frame loop.
//!
//! The loop is: advance time -> collect cook roots -> pull -> draw. Roots are
//! the perform-output operator, every node whose viewer flag is on and on
//! screen, and render/cook roots. That visibility rule is what makes the
//! demand-driven engine
//! visible to the user: pan a heavy branch off screen and it stops costing
//! anything (PLAN.md §4).

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use egui::TextureId;
use otd_core::{
    copy_selection, paste, CookContext, CookEngine, Graph, History, NodeClipboard, NodeId,
    OpRegistry, Project, Value,
};
use otd_engine::Engines;
use otd_gpu::GpuContext;

use crate::canvas::{CanvasView, DragState, PendingParamDrop};

pub struct OtdApp {
    pub graph: Graph,
    pub registry: OpRegistry,
    pub cook: CookEngine,
    pub engines: Engines,
    pub render_state: eframe::egui_wgpu::RenderState,

    pub time: CookContext,
    pub playing: bool,
    /// The looping range, in seconds. Time wraps to the start when it passes
    /// the end, which is what makes a timeline a *loop* rather than a stopwatch.
    pub loop_range: (f64, f64),
    pub looping: bool,
    /// Set while the playhead is being dragged, so the transport does not
    /// fight the scrub.
    scrubbing: bool,
    last_instant: Instant,
    pub smoothed_fps: f64,
    pub smoothed_cook_ms: f64,

    /// The node the parameter panel is showing, and the referent of "it" in
    /// an assistant request. The *primary* of the selection rather than a
    /// second idea of what is selected: whenever `selection` is non-empty this
    /// is one of its members, and clearing one clears the other. Keeping it
    /// separate is what lets a five-node selection still have a parameter
    /// panel — the alternative is showing nothing the moment you select two
    /// things, which is worse.
    pub selected: Option<NodeId>,
    /// Everything selected, primary included. Delete and drag act on all of
    /// it; the parameter panel acts on `selected`.
    pub selection: BTreeSet<NodeId>,
    pub selected_wire: Option<(NodeId, usize)>,
    /// Legacy cache; use [`Self::perform_ui`] (Window Placement → Perform UI container).
    pub viewer: Option<NodeId>,
    /// The component whose network is on screen. Entering a component is
    /// just changing this — the graph is one tree throughout.
    pub current: NodeId,
    pub view: CanvasView,
    network_views: HashMap<NodeId, CanvasView>,
    /// Where the network editor was drawn last frame. Drag-and-drop needs it
    /// to turn a pointer position into a place in the network.
    pub canvas_rect: egui::Rect,
    pub drag: Option<DragState>,
    pub(crate) node_drag_start: Option<Vec<(NodeId, [f32; 2])>>,
    pub(crate) pending_param_drop: Option<PendingParamDrop>,
    graph_commands: Vec<graph_commands::GraphCommand>,
    pending_media_drop: Option<(Vec<PathBuf>, Option<egui::Pos2>)>,
    pub create_dialog: Option<CreateDialog>,
    /// Screen rect of the Add operator window, for blocking canvas zoom while
    /// the pointer is over it.
    pub create_dialog_rect: Option<egui::Rect>,
    pub show_perf: bool,
    /// The performance monitor window.
    pub show_monitor: bool,
    pub show_textport: bool,
    pub textport: crate::textport::Textport,
    pub code_lint: crate::code_lint::CodeLintCache,
    /// Perform mode: the editor is gone and the main window shows only the
    /// output. F1 in, F1 or Escape out.
    pub perform: bool,
    /// Edit the Perform UI container (network + parameters) without leaving
    /// perform mode — F1 returns to fullscreen preview.
    pub perform_edit: bool,
    /// What we last told the OS about the main window, so the fullscreen
    /// command is sent on the edge rather than every frame.
    perform_applied: bool,
    /// The Movie File Out the Record button drives, once it has made one.
    pub recorder: Option<NodeId>,
    /// A second window showing only the viewer output — the projector feed.
    pub output_window: bool,
    pub output_fullscreen: bool,
    pub show_window_placement: bool,
    pub main_perform_window: Option<NodeId>,
    pub main_perform_window_path: String,
    pub main_perform_ui_path: String,
    pub start_in_perform_mode: bool,
    pub designer_custom_position: bool,
    pub designer_x: f32,
    pub designer_y: f32,
    pub designer_w: f32,
    pub designer_h: f32,
    pub(crate) designer_position_pending: bool,
    /// Set from inside the output viewport when the user closes it. The
    /// viewport callback is `'static` and cannot reach `self`.
    output_closed: Arc<AtomicBool>,
    pub(crate) window_close_flags: HashMap<NodeId, Arc<AtomicBool>>,
    pub(crate) window_open_anchors: HashMap<NodeId, egui::Pos2>,
    pub(crate) window_was_open: HashMap<NodeId, bool>,
    pub open_container_views: BTreeSet<NodeId>,
    pub callback_embedded: BTreeSet<NodeId>,
    pub viewer_top_pan: HashMap<NodeId, egui::Vec2>,
    pub viewer_top_zoom: HashMap<NodeId, f32>,
    pub viewer_dat_scroll: HashMap<NodeId, egui::Vec2>,

    /// Mouse and keyboard, sampled each frame for the input CHOPs.
    pub input_state: otd_chop::InputState,
    /// Draft state for the "add a component parameter" row.
    pub new_param_name: String,
    pub new_param_type: String,
    /// Active inspector tab per node (TouchDesigner-style pages).
    pub param_inspector_page: HashMap<NodeId, String>,

    thumbs: HashMap<NodeId, (TextureId, u64)>,
    pending_texture_sync: Vec<NodeId>,
    /// Nodes that were on screen last frame — the visible cook roots.
    pub visible: Vec<NodeId>,

    pub history: History,

    /// The assistant panel: describe a patch, get operators.
    pub assistant: crate::assistant::Assistant,

    pub project_path: Option<PathBuf>,
    pub status: String,
    pub cook_error: Option<String>,
    /// Last channel expression copied from the panel or a CHOP tile (for Apply).
    pub channel_expr_clipboard: String,
    node_clipboard: NodeClipboard,
    paste_count: u32,

    pub settings: crate::settings::AppSettings,
    pub show_settings: bool,
    pub settings_draft: Option<crate::settings::AppSettings>,
    blur_params_on_select: bool,
    pub(crate) skip_canvas_deselect_once: bool,
    pub canvas_rename: Option<(NodeId, String)>,
    document_dirty: bool,
    show_close_save_prompt: bool,
    playing_before_minimize: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub enum WireDrop {
    FromOutput {
        from: NodeId,
        output: usize,
    },
    ToInput {
        dst: NodeId,
        input: usize,
    },
}

#[derive(Clone, Debug)]
pub struct CreateDialog {
    pub filter: String,
    pub family: otd_core::Family,
    pub world_pos: egui::Vec2,
    pub focus: bool,
    pub connect_from: Option<NodeId>,
    pub wire_drop: Option<WireDrop>,
}

mod cook;
mod graph_commands;
mod ui;

pub(crate) use ui::{draw_panel_comp, draw_texture_stretch};

impl OtdApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        let render_state = cc
            .wgpu_render_state
            .clone()
            .ok_or("OpenTouchDesigner needs the wgpu backend")?;

        // The cook engine renders into the same device egui draws with, so a
        // node thumbnail is the operator's real texture — no copy, no readback.
        let gpu = GpuContext::new(render_state.device.clone(), render_state.queue.clone());
        let engines = Engines::new(gpu);
        let registry = otd_engine::registry();
        let settings = crate::settings::AppSettings::load();

        let graph = Graph::new();
        let root = graph.root();
        let mut app = OtdApp {
            graph,
            registry,
            cook: CookEngine::new(),
            engines,
            render_state,
            time: CookContext::default(),
            playing: true,
            loop_range: (0.0, 10.0),
            looping: false,
            scrubbing: false,
            last_instant: Instant::now(),
            smoothed_fps: 60.0,
            smoothed_cook_ms: 0.0,
            selected: None,
            selection: BTreeSet::new(),
            selected_wire: None,
            viewer: None,
            current: root,
            view: CanvasView::default(),
            network_views: HashMap::new(),
            canvas_rect: egui::Rect::NOTHING,
            drag: None,
            node_drag_start: None,
            pending_param_drop: None,
            graph_commands: Vec::new(),
            pending_media_drop: None,
            create_dialog: None,
            create_dialog_rect: None,
            show_perf: true,
            show_monitor: false,
            show_textport: false,
            textport: crate::textport::Textport::default(),
            code_lint: crate::code_lint::CodeLintCache::default(),
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
            output_closed: Arc::new(AtomicBool::new(false)),
            window_close_flags: HashMap::new(),
            window_open_anchors: HashMap::new(),
            window_was_open: HashMap::new(),
            open_container_views: BTreeSet::new(),
            callback_embedded: BTreeSet::new(),
            viewer_top_pan: HashMap::new(),
            viewer_top_zoom: HashMap::new(),
            viewer_dat_scroll: HashMap::new(),
            input_state: otd_chop::InputState::default(),
            new_param_name: String::new(),
            new_param_type: "float".to_string(),
            param_inspector_page: HashMap::new(),
            thumbs: HashMap::new(),
            pending_texture_sync: Vec::new(),
            visible: Vec::new(),
            history: History::new(settings.undo_limit),
            assistant: crate::assistant::Assistant::default(),
            project_path: None,
            status: String::new(),
            cook_error: None,
            channel_expr_clipboard: String::new(),
            node_clipboard: NodeClipboard::default(),
            paste_count: 0,
            settings,
            show_settings: false,
            settings_draft: None,
            blur_params_on_select: false,
            skip_canvas_deselect_once: false,
            canvas_rename: None,
            document_dirty: false,
            show_close_save_prompt: false,
            playing_before_minimize: None,
        };
        app.apply_settings(&cc.egui_ctx);
        app.textport
            .info(0, "Textport — cook errors, callbacks, and Python (Enter to run).");
        if let Some(e) = app.engines.python_error() {
            app.textport.error(0, e);
        }
        Ok(app)
    }

    pub fn apply_settings(&mut self, ctx: &egui::Context) {
        self.show_perf = self.settings.show_node_cook_time;
        self.history.set_limit(self.settings.undo_limit);
        ctx.set_theme(self.settings.ui_theme.to_egui());
        ctx.all_styles_mut(|style| {
            style.debug.warn_if_rect_changes_id = false;
        });
    }

    pub(crate) fn sync_viewport_playback(&mut self, ctx: &egui::Context) {
        if !self.settings.stop_playing_when_minimized {
            self.playing_before_minimize = None;
            return;
        }
        let minimized = ctx.input(|i| i.viewport().minimized == Some(true));
        if minimized {
            if self.playing_before_minimize.is_none() && self.playing {
                self.playing_before_minimize = Some(true);
                self.playing = false;
            }
        } else if let Some(was_playing) = self.playing_before_minimize.take() {
            if was_playing {
                self.playing = true;
            }
        }
    }

    pub(crate) fn handle_main_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if self.show_close_save_prompt {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            return;
        }
        if self.settings.prompt_save_on_exit && self.document_dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.show_close_save_prompt = true;
        }
    }

    pub(crate) fn close_save_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_close_save_prompt {
            return;
        }
        let mut save = false;
        let mut discard = false;
        let mut dismiss = false;
        egui::Window::new("Save changes?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Save changes before closing?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        save = true;
                    }
                    if ui.button("Don't Save").clicked() {
                        discard = true;
                    }
                    if ui.button("Cancel").clicked() {
                        dismiss = true;
                    }
                });
            });
        if save {
            let path = self.project_path.clone();
            self.save(path);
            if !self.document_dirty {
                self.show_close_save_prompt = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if discard {
            self.document_dirty = false;
            self.show_close_save_prompt = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if dismiss {
            self.show_close_save_prompt = false;
        }
    }

    pub fn apply_startup(&mut self) {
        use crate::settings::StartupMode;
        match self.settings.startup_mode {
            StartupMode::DefaultExample => {
                let name = if otd_engine::demo::NAMES
                    .iter()
                    .any(|&n| n == self.settings.startup_demo.as_str())
                {
                    self.settings.startup_demo.clone()
                } else {
                    "tunnel".to_string()
                };
                self.load_demo(&name);
            }
            StartupMode::Blank => self.load_blank(),
            StartupMode::CustomFile => {
                let path = self.settings.startup_custom_path.clone();
                if path.as_os_str().is_empty() {
                    self.load_blank();
                    self.status = "Custom startup file not set — blank project".into();
                } else if path.is_file() {
                    self.open(Some(path));
                } else {
                    self.load_blank();
                    self.status = format!("Startup file not found: {}", path.display());
                }
            }
        }
    }

    pub fn snap_world_pos(&self, pos: [f32; 2]) -> [f32; 2] {
        if self.settings.snap_nodes_to_grid {
            crate::canvas::grid::snap_world_pos(pos)
        } else {
            pos
        }
    }

    pub fn try_set_node_name(&mut self, id: NodeId, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let parent = self.graph.node(id).parent.unwrap_or(self.graph.root());
        if self.graph.name_taken(parent, name) && self.graph.node(id).name != name {
            self.status = format!("name `{name}` already used here");
            return;
        }
        self.edit("rename");
        self.graph.node_mut_quiet(id).name = name.to_string();
        self.history.end_gesture();
    }

    pub fn commit_canvas_rename(&mut self) {
        let Some((id, name)) = self.canvas_rename.take() else {
            return;
        };
        self.try_set_node_name(id, &name);
    }

    pub fn apply_new_project_defaults(&mut self) {
        self.set_project_fps(self.settings.default_project_fps);
        self.playing = self.settings.start_playing;
        self.looping = self.settings.default_looping;
        let start = self.settings.default_loop_start;
        let end = if self.settings.default_loop_end > self.settings.default_loop_start {
            self.settings.default_loop_end
        } else {
            self.settings.default_loop_start + 0.1
        };
        self.loop_range = (start, end);
    }

    /// Load one of the built-in patches (see `otd_gpu::demo`). The editor
    /// opens on `starter`, which is the Phase 0 exit criterion made visible.
    pub fn load_demo(&mut self, name: &str) {
        let Some((graph, _out)) = otd_engine::demo::by_name(name, &self.registry) else {
            self.load_blank();
            self.status = format!("no built-in patch called `{name}`");
            return;
        };
        self.assistant.reset_context();
        self.graph = graph;
        self.engines.reset();
        self.cook.reset();
        self.thumbs.clear();
        self.history.clear();
        self.project_path = None;
        self.document_dirty = false;
        self.time = CookContext::default();
        self.current = self.graph.root();
        self.view = CanvasView::default();
        self.network_views.clear();
        self.viewer = self
            .graph
            .walk()
            .into_iter()
            .rev()
            .find(|&id| self.graph.node(id).family == otd_core::Family::Comp);
        self.clear_selection();
        if let Some(first) = self.graph.walk().into_iter().nth(1) {
            self.select_only(first);
        }
        self.status = format!("{name} patch");
        self.apply_new_project_defaults();
    }

    pub fn load_blank(&mut self) {
        self.assistant.reset_context();
        self.graph = Graph::new();
        self.engines.reset();
        self.cook.reset();
        self.thumbs.clear();
        self.history.clear();
        self.project_path = None;
        self.document_dirty = false;
        self.time = CookContext::default();
        self.current = self.graph.root();
        self.view = CanvasView::default();
        self.network_views.clear();
        self.viewer = None;
        self.clear_selection();
        self.status = "blank project".into();
        self.apply_new_project_defaults();
    }

    pub fn set_project_fps(&mut self, fps: f64) {
        let fps = fps.clamp(1.0, 240.0);
        self.time.fps = fps;
        self.time.frame = (self.time.time * fps).round() as i64;
    }

    // -------------------------------------------------------------- cooking

    pub fn perform_ui(&self) -> Option<NodeId> {
        crate::window_comp::resolve_perform_ui(&self.graph, &self.main_perform_ui_path)
    }

    pub fn perform_preview_only(&self) -> bool {
        self.perform && !self.perform_edit
    }

    pub fn focus_perform_ui_for_edit(&mut self) {
        let Some(comp) = self.perform_ui() else {
            return;
        };
        if self.graph.node(comp).family == otd_core::Family::Comp {
            self.queue_enter_network(comp);
        }
        self.select_only(comp);
    }

    pub fn leave_perform_for_edit(&mut self) {
        self.perform = false;
        self.perform_edit = false;
        self.focus_perform_ui_for_edit();
    }

    pub fn view_panel_comp(&mut self, id: NodeId) {
        if !self.graph.contains(id) || self.graph.node(id).family != otd_core::Family::Comp {
            return;
        }
        if !otd_engine::panel::is_panel_type(&self.graph.node(id).op_type) {
            return;
        }
        if self.graph.node(id).op_type == otd_engine::panel::CONTAINER {
            self.open_container_views.insert(id);
            self.viewer = Some(id);
            return;
        }
        self.set_perform_ui(id);
        self.perform = true;
        self.perform_edit = false;
    }

    pub fn save_component(&mut self) {
        let Some(id) = self.selected.filter(|s| self.graph.contains(*s)) else {
            return;
        };
        if self.graph.node(id).family != otd_core::Family::Comp {
            self.status = "select a component first".into();
            return;
        }
        let name = self.graph.node(id).name.clone();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenTouchDesigner component", &["otdc"])
            .set_file_name(format!("{name}.otdc"))
            .save_file()
        else {
            return;
        };
        match otd_core::Component::from_graph(&self.graph, id, &self.registry) {
            Some(c) => match c.save(&path) {
                Ok(()) => self.status = format!("Saved component {}", path.display()),
                Err(e) => self.status = format!("Save failed: {e}"),
            },
            None => self.status = "could not read that component".into(),
        }
    }

    /// Bring a `.otdc` into the current network, linked to its file so later
    /// edits to the shared definition arrive here.
    pub fn import_component(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenTouchDesigner component", &["otdc"])
            .pick_file()
        else {
            return;
        };
        self.import_component_at(&path);
    }

    /// The same, for a `.otdc` that arrived by being dropped on the network.
    pub fn import_component_at(&mut self, path: &std::path::Path) -> Option<NodeId> {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "component".into());
        let def = self.registry.get(otd_engine::comp::BASE).cloned()?;
        let parent = self.current;
        self.edit("import component");
        self.history.end_gesture();
        let id = self.graph.create(parent, &def, Some(&name)).ok()?;
        match self
            .graph
            .attach_external(id, &path.to_string_lossy(), &self.registry)
        {
            Ok(()) => {
                self.select_only(id);
                self.status = format!("Imported {}", path.display());
                Some(id)
            }
            Err(e) => {
                let _ = self.graph.remove(id);
                self.status = format!("Import failed: {e}");
                None
            }
        }
    }

    /// Copy the project and everything it references into one folder.
    ///
    /// A project on an authoring machine points at shared `.otdc` files that
    /// live wherever the artist keeps them. A show machine has none of those
    /// directories. Exporting copies them in and rewrites the references to be
    /// relative, so the folder can be moved anywhere and still open.
    pub fn export_bundle(&mut self) {
        let name = self
            .project_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "show".into());
        let Some(dir) = rfd::FileDialog::new()
            .set_file_name(&name)
            .set_title("Export bundle into a new folder")
            .pick_folder()
        else {
            return;
        };
        match otd_core::bundle::export(&self.graph, &self.registry, self.time.fps, &dir, &name) {
            Ok(bundle) => {
                self.status = format!(
                    "Exported {} — {} component(s), {} media file(s){}",
                    bundle.project.display(),
                    bundle.components.len(),
                    bundle.media.len(),
                    if bundle.missing.is_empty() {
                        String::new()
                    } else {
                        // Loud, not silent: a bundle missing a component will
                        // fail on the show machine, not here.
                        format!(
                            ", {} MISSING: {}",
                            bundle.missing.len(),
                            bundle
                                .missing
                                .iter()
                                .map(|(path, file, _)| format!("{path} -> {file}"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                );
            }
            Err(e) => self.status = format!("Export failed: {e}"),
        }
    }

    /// Load an `.fs` ISF shader onto a GLSL TOP.
    ///
    /// The point of ISF is that thousands of effects already exist in it. This
    /// turns one of them into an ordinary node with ordinary parameters, so it
    /// can be bound, exported to and saved like anything else.
    pub fn import_isf(&mut self, id: NodeId) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("ISF shader", &["fs", "isf", "frag"])
            .pick_file()
        else {
            return;
        };
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                self.status = format!("Import failed: {e}");
                return;
            }
        };
        match otd_gpu::isf::import(&source) {
            Ok(isf) => {
                self.edit("import ISF");
                self.history.end_gesture();
                let count = isf.params.len();
                otd_gpu::isf::apply(&mut self.graph, id, &isf);
                let name = path.file_stem().unwrap_or_default().to_string_lossy();
                self.status = format!("Imported {name} — {count} parameter(s)");
            }
            Err(e) => self.status = format!("Import failed: {e}"),
        }
    }

    // --------------------------------------------------------- selection
    //
    // Every path in and out of the selection goes through these, so the
    // invariant — `selected` is in `selection`, or both are empty — holds by
    // construction rather than by everybody remembering.

    /// Click with no modifier: this node and nothing else.
    pub fn select_only(&mut self, id: NodeId) {
        self.selection.clear();
        self.selection.insert(id);
        self.selected = Some(id);
        self.selected_wire = None;
        self.blur_params_on_select = self.settings.blur_params_on_select;
    }

    /// Shift- or Cmd-click: add if absent, remove if present.
    pub fn select_toggle(&mut self, id: NodeId) {
        if self.selection.remove(&id) {
            // Removing the primary promotes whatever is left, so a selection
            // that still has members still has a parameter panel.
            if self.selected == Some(id) {
                self.selected = self.selection.iter().next().copied();
            }
        } else {
            self.selection.insert(id);
            self.selected = Some(id);
        }
    }

    /// Cmd/Ctrl+A: everything in the network on screen.
    ///
    /// The network on screen, not the whole graph — the canvas is the thing
    /// being selected into, and reaching inside components you are not
    /// looking at would make the next Delete much bigger than it looked.
    pub fn select_all(&mut self) {
        self.selection = self.graph.node(self.current).children.iter().copied().collect();
        // Keep the primary if it survived, so Cmd+A does not swap the
        // parameter panel out from under you.
        if !self.selected.map(|s| self.selection.contains(&s)).unwrap_or(false) {
            self.selected = self.selection.iter().next().copied();
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.selected = None;
        self.selected_wire = None;
    }

    pub fn is_selected(&self, id: NodeId) -> bool {
        self.selection.contains(&id)
    }

    fn clipboard_keys(&mut self, ctx: &egui::Context) {
        crate::canvas::input::operator_clipboard_keys(self, ctx);
    }

    pub fn has_operator_clipboard(&self) -> bool {
        !self.node_clipboard.nodes.is_empty()
    }

    pub fn copy_selection(&mut self, ctx: &egui::Context) {
        let ids: Vec<NodeId> = if !self.selection.is_empty() {
            self.selection.iter().copied().collect()
        } else {
            self.selected.into_iter().collect()
        };
        if ids.is_empty() {
            return;
        }
        self.node_clipboard = copy_selection(&self.graph, self.current, &ids, &self.registry);
        if self.node_clipboard.nodes.is_empty() {
            self.status = "Nothing to copy (locked or outside this network)".into();
        } else {
            self.paste_count = 0;
            self.status = format!("Copied {} operator(s)", self.node_clipboard.nodes.len());
            if let Some(text) = otd_core::encode_operator_clipboard(&self.node_clipboard) {
                ctx.copy_text(text);
            }
        }
    }

    pub fn cut_selection(&mut self, ctx: &egui::Context) {
        self.copy_selection(ctx);
        if !self.node_clipboard.nodes.is_empty() {
            self.queue_delete_selection();
        }
    }

    pub fn load_operator_clipboard_from_text(&mut self, text: &str) -> bool {
        let Some(clip) = otd_core::decode_operator_clipboard(text) else {
            return false;
        };
        if clip.nodes.is_empty() {
            return false;
        }
        self.node_clipboard = clip;
        self.paste_count = 0;
        true
    }

    pub fn apply_paste_clipboard(&mut self, origin: [f32; 2]) {
        if self.node_clipboard.nodes.is_empty() {
            return;
        }
        self.paste_count += 1;
        let nudge = (self.paste_count.saturating_sub(1) as f32) * 24.0;
        let at = [origin[0] + nudge, origin[1] + nudge];
        self.edit("paste");
        self.history.end_gesture();
        match paste(
            &mut self.graph,
            self.current,
            &self.registry,
            &self.node_clipboard,
            at,
        ) {
            Ok(ids) => {
                self.clear_selection();
                for id in &ids {
                    self.selection.insert(*id);
                }
                self.selected = ids.last().copied();
                self.status = format!("Pasted {} operator(s)", ids.len());
            }
            Err(e) => self.status = format!("Paste failed: {e}"),
        }
        self.repair_app_graph_pointers();
    }

    fn selection_keys(&mut self, ctx: &egui::Context) {
        let cmd_a = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A));
        if !cmd_a {
            return;
        }
        let over_network = ctx
            .pointer_latest_pos()
            .map(|p| self.canvas_rect.contains(p))
            .unwrap_or(true);
        if ctx.text_edit_focused() && !over_network {
            return;
        }
        if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A)) {
            return;
        }
        crate::canvas::input::release_param_focus(ctx);
        self.select_all();
    }

    fn flag_keys(&mut self, ctx: &egui::Context) {
        if self.perform_preview_only() {
            return;
        }
        let count = self.selection.len();
        if count == 0 && self.selected.is_none() {
            return;
        }
        if ctx.text_edit_focused() && count <= 1 {
            return;
        }
        let flag = ctx.input_mut(|i| {
            if i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt {
                return None;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::V) {
                Some(crate::canvas::input::SelectionFlag::Viewer)
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::D) {
                Some(crate::canvas::input::SelectionFlag::Display)
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::L) {
                Some(crate::canvas::input::SelectionFlag::Lock)
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::B) {
                Some(crate::canvas::input::SelectionFlag::Bypass)
            } else {
                None
            }
        });
        if let Some(flag) = flag {
            crate::canvas::input::toggle_flag_on_selection(self, flag);
        }
    }

    // -------------------------------------------------------- navigation

    fn switch_network(&mut self, id: NodeId) {
        if id == self.current || !self.graph.contains(id) {
            return;
        }
        self.network_views.insert(self.current, self.view);
        self.current = id;
        self.view = self.network_views.get(&id).copied().unwrap_or_default();
    }

    /// Step inside a component. Anything else is ignored, so a stray
    /// double-click on an operator does not lose your place.
    pub fn apply_enter_network(&mut self, id: NodeId) {
        if self.graph.get(id).map(|n| n.family) == Some(otd_core::Family::Comp) {
            self.switch_network(id);
            self.clear_selection();
        }
    }

    /// Step out to the parent network.
    pub fn apply_leave_network(&mut self) {
        if let Some(parent) = self.graph.get(self.current).and_then(|n| n.parent) {
            let was = self.current;
            self.switch_network(parent);
            self.select_only(was);
        }
    }

    fn navigation_keys(&mut self, ctx: &egui::Context) {
        if self.perform_preview_only() || ctx.text_edit_focused() {
            return;
        }
        let mut leave = false;
        let mut enter = None;
        ctx.input_mut(|i| {
            if i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt {
                return;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::U) {
                leave = true;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::I)
            {
                enter = self.selected.filter(|id| {
                    self.graph.contains(*id)
                        && self.graph.node(*id).family == otd_core::Family::Comp
                });
            } else if self.settings.enable_playbar_shortcuts
                && i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
            {
                self.playing = !self.playing;
            }
        });
        if leave {
            self.queue_leave_network();
        } else if let Some(id) = enter {
            self.queue_enter_network(id);
        }
    }

    // -------------------------------------------------------------- history

    /// Cmd/Ctrl+Z and Cmd/Ctrl+Shift+Z (or Cmd+Y).
    ///
    /// Handled here rather than on the canvas so undo works with the parameter
    /// panel focused and in perform mode too. While a text field has focus,
    /// egui's own character-level undo is the right behaviour and wins.
    fn history_keys(&mut self, ctx: &egui::Context) {
        if ctx.text_edit_focused() {
            return;
        }
        let (undo, redo) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            (
                cmd && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                cmd && ((i.modifiers.shift && i.key_pressed(egui::Key::Z))
                    || i.key_pressed(egui::Key::Y)),
            )
        });
        if redo {
            self.redo();
        } else if undo {
            self.undo();
        }
    }

    pub(crate) fn open_create_operator_dialog(&mut self, world_pos: egui::Vec2) {
        let family = self
            .selected
            .filter(|id| self.graph.contains(*id))
            .map(|id| self.graph.node(id).family)
            .unwrap_or(otd_core::Family::Top);
        self.create_dialog = Some(CreateDialog {
            filter: String::new(),
            family,
            world_pos,
            focus: true,
            connect_from: self.selected,
            wire_drop: None,
        });
        self.create_dialog_rect = None;
    }

    pub(crate) fn open_create_operator_dialog_for_wire(
        &mut self,
        world_pos: egui::Vec2,
        wire: WireDrop,
    ) {
        use crate::canvas::ports::output_port_family;

        let family = match wire {
            WireDrop::FromOutput { from, output } => output_port_family(&self.graph, from, output),
            WireDrop::ToInput { dst, input } => self
                .graph
                .node(dst)
                .input_families
                .get(input)
                .copied()
                .unwrap_or(self.graph.node(dst).family),
        };
        self.create_dialog = Some(CreateDialog {
            filter: String::new(),
            family,
            world_pos,
            focus: true,
            connect_from: None,
            wire_drop: Some(wire),
        });
        self.create_dialog_rect = None;
    }

    pub(crate) fn connect_created_node(&mut self, new_id: NodeId, dialog: &CreateDialog) {
        if let Some(wire) = dialog.wire_drop {
            let result = match wire {
                WireDrop::FromOutput { from, output } => {
                    self.graph.connect_from_output(from, output, new_id, 0)
                }
                WireDrop::ToInput { dst, input } => {
                    self.graph.connect_from_output(new_id, 0, dst, input)
                }
            };
            match result {
                Ok(()) => self.status.clear(),
                Err(e) => self.status = e.to_string(),
            }
        } else if let Some(src) = dialog.connect_from.filter(|s| self.graph.contains(*s)) {
            match self.graph.connect(src, new_id, 0) {
                Ok(()) => self.status.clear(),
                Err(e) => self.status = e.to_string(),
            }
        }
    }

    /// Tab opens the operator picker. Handled before the menu bar so egui's
    /// focus cycle (Edit → File → Media …) does not swallow it; only a real
    /// text field should block the shortcut.
    fn create_operator_tab_key(&mut self, ctx: &egui::Context) {
        if self.perform_preview_only() || ctx.text_edit_focused() {
            return;
        }
        let tab = ctx.input(|i| i.key_pressed(egui::Key::Tab) && !i.modifiers.any());
        if !tab {
            return;
        }
        ctx.memory_mut(|mem| mem.move_focus(egui::FocusDirection::None));
        let pointer = ctx
            .pointer_latest_pos()
            .unwrap_or_else(|| self.canvas_rect.center());
        let origin = self.canvas_rect.min;
        let world_pos = if self.canvas_rect.is_positive() {
            self.view.to_world(origin, pointer)
        } else {
            egui::Vec2::ZERO
        };
        self.open_create_operator_dialog(world_pos);
    }

    /// Cmd/Ctrl+S, and Cmd/Ctrl+Shift+S for Save As.
    ///
    /// Deliberately *not* behind the "is a text field focused" guard that
    /// history uses. Undo defers to egui's character-level undo while you are
    /// typing, which is right; saving has no such conflict, and the moment
    /// somebody most wants Cmd+S is right after typing a name into a
    /// parameter.
    fn file_keys(&mut self, ctx: &egui::Context) {
        let (save, save_as) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            (
                cmd && !i.modifiers.shift && i.key_pressed(egui::Key::S),
                cmd && i.modifiers.shift && i.key_pressed(egui::Key::S),
            )
        });
        if save_as {
            self.save_as();
        } else if save {
            // `save` falls back to asking for a path when the project has
            // never been written, so this is Save As for an untitled patch
            // without being a second shortcut.
            let path = self.project_path.clone();
            self.save(path);
        }
    }

    /// Record the graph before a change, so it can be undone.
    ///
    /// `tag` names what is being edited. While it stays the same the edits are
    /// one entry, which is what makes dragging a slider undo as the gesture it
    /// was rather than as the sixty frames it took.
    pub fn edit(&mut self, tag: &str) {
        self.document_dirty = true;
        self.history.checkpoint(&self.graph, tag);
    }

    pub fn undo(&mut self) {
        match self.history.undo(&self.graph) {
            Some(graph) => {
                self.restore(graph);
                self.status = "Undo".into();
            }
            None => self.status = "nothing to undo".into(),
        }
    }

    pub fn redo(&mut self) {
        match self.history.redo(&self.graph) {
            Some(graph) => {
                self.restore(graph);
                self.status = "Redo".into();
            }
            None => self.status = "nothing to redo".into(),
        }
    }

    /// Swap in a restored graph and repair everything that pointed into the
    /// old one.
    ///
    /// Node ids survive a snapshot, so the selection and the viewer usually
    /// still resolve — but not if the step being undone was a delete, or the
    /// step being redone was a create. Those are exactly the cases that would
    /// otherwise leave a dangling id in the cook roots.
    fn restore(&mut self, graph: Graph) {
        self.graph = graph;
        self.repair_app_graph_pointers();
        self.drag = None;
    }

    fn collect_subtree(&self, root: NodeId) -> Vec<NodeId> {
        if !self.graph.contains(root) {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            out.push(id);
            for &c in self.graph.children(id) {
                stack.push(c);
            }
        }
        out
    }

    fn rehome_current_before_remove(&mut self, roots: &[NodeId]) {
        for &root in roots {
            if !self.graph.contains(root) {
                continue;
            }
            if self.current != root
                && !crate::canvas::comp_display::is_descendant_of(&self.graph, root, self.current)
            {
                continue;
            }
            let parent = self
                .graph
                .node(root)
                .parent
                .filter(|p| self.graph.contains(*p))
                .unwrap_or_else(|| self.graph.root());
            self.network_views.insert(self.current, self.view);
            self.current = parent;
            self.view = self
                .network_views
                .get(&parent)
                .copied()
                .unwrap_or_default();
            return;
        }
    }

    fn free_thumbnail(&mut self, id: NodeId) {
        if let Some((tid, _)) = self.thumbs.remove(&id) {
            self.render_state
                .renderer
                .write()
                .free_texture(&tid);
        }
        self.pending_texture_sync.retain(|pending| *pending != id);
    }

    fn forget_node_runtime(&mut self, id: NodeId) {
        self.engines.forget(id);
        self.cook.forget(id);
        self.free_thumbnail(id);
        self.viewer_top_pan.remove(&id);
        self.viewer_top_zoom.remove(&id);
        self.viewer_dat_scroll.remove(&id);
        self.param_inspector_page.remove(&id);
        self.callback_embedded.remove(&id);
        self.window_close_flags.remove(&id);
        self.window_open_anchors.remove(&id);
        self.window_was_open.remove(&id);
    }

    fn repair_app_graph_pointers(&mut self) {
        self.selected = self.selected.filter(|id| self.graph.contains(*id));
        self.selection.retain(|id| self.graph.contains(*id));
        if self.selected.is_none() {
            self.selected = self.selection.iter().next().copied();
        }
        self.viewer = self.viewer.filter(|id| {
            self.graph.contains(*id) && self.graph.node(*id).family == otd_core::Family::Comp
        });
        if !self.main_perform_ui_path.is_empty()
            && self.graph.find(&self.main_perform_ui_path).is_none()
        {
            self.main_perform_ui_path.clear();
        }
        if !self.graph.contains(self.current) {
            self.current = self.graph.root();
            self.view = self
                .network_views
                .get(&self.current)
                .copied()
                .unwrap_or_default();
        }
        self.visible.retain(|id| self.graph.contains(*id));
        self.network_views.retain(|id, _| self.graph.contains(*id));
        self.open_container_views
            .retain(|id| self.graph.contains(*id));
    }

    // ---------------------------------------------------------- graph edits

    pub(crate) fn apply_disconnect(&mut self, dst: NodeId, input: usize) {
        if !self.graph.contains(dst) || self.graph.node(dst).flags.lock {
            return;
        }
        self.edit("unwire");
        self.history.end_gesture();
        let _ = self.graph.disconnect(dst, input);
        if self.selected_wire == Some((dst, input)) {
            self.selected_wire = None;
        }
        self.status.clear();
    }

    pub(crate) fn apply_connect(
        &mut self,
        from: NodeId,
        output: usize,
        dst: NodeId,
        input: usize,
    ) {
        self.edit("wire");
        self.history.end_gesture();
        match self.graph.connect_from_output(from, output, dst, input) {
            Ok(()) => self.status.clear(),
            Err(e) => self.status = e.to_string(),
        }
    }

    /// Delete everything selected, as one undo step.
    pub fn delete_selected(&mut self) {
        if let Some((dst, input)) = self.selected_wire {
            if self.graph.contains(dst) && !self.graph.node(dst).flags.lock {
                self.apply_disconnect(dst, input);
            }
            return;
        }
        if self.selection.is_empty() {
            return;
        }
        let doomed: Vec<NodeId> = self
            .selection
            .iter()
            .copied()
            .filter(|id| self.graph.contains(*id) && !self.graph.node(*id).flags.lock)
            .collect();
        if doomed.is_empty() {
            self.status = "Selection is locked".into();
            return;
        }
        self.edit("delete");
        self.history.end_gesture();
        for id in &doomed {
            self.selection.remove(id);
        }
        self.selected = self
            .selected
            .filter(|s| self.selection.contains(s))
            .or_else(|| self.selection.iter().next().copied());
        self.delete_all(&doomed);
    }

    /// Remove a set of nodes as one gesture, healing the chains they sat in.
    ///
    /// The splices are all worked out first, against the graph as it stands.
    /// Healing node by node reads a graph the previous removal already
    /// changed: deleting `b` and `c` from `a -> b -> c -> d` would look up
    /// `c`'s input after `b` had gone, find nothing, and leave `d` empty —
    /// which is a black texture and looks like the delete broke the patch.
    fn delete_all(&mut self, doomed: &[NodeId]) {
        let live: Vec<NodeId> = doomed
            .iter()
            .copied()
            .filter(|id| self.graph.contains(*id))
            .collect();
        let splices = otd_ai::patch::splices_for(&self.graph, &live);
        self.rehome_current_before_remove(&live);
        let mut removed = Vec::new();
        for id in &live {
            removed.extend(self.collect_subtree(*id));
        }
        removed.sort_unstable();
        removed.dedup();
        for id in &live {
            let _ = self.graph.remove(*id);
            if self.perform_ui() == Some(*id) {
                self.main_perform_ui_path.clear();
                self.viewer = None;
            }
        }
        for id in removed {
            self.forget_node_runtime(id);
        }
        for (src, consumer, slot) in splices {
            let _ = self.graph.connect(src, consumer, slot);
        }
        self.repair_app_graph_pointers();
        self.drag = None;
    }

    /// Empty the network on screen — the `/clear` command and the File menu.
    ///
    /// One undo, like everything else the assistant does, because the whole
    /// point of it is to be the fast way back from a patch that went wrong.
    pub fn clear_network(&mut self) -> usize {
        self.assistant.reset_context();
        let doomed: Vec<NodeId> = self.graph.node(self.current).children.clone();
        if doomed.is_empty() {
            return 0;
        }
        self.edit("clear");
        self.history.end_gesture();
        self.clear_selection();
        self.delete_all(&doomed);
        self.main_perform_ui_path.clear();
        self.viewer = None;
        doomed.len()
    }

    pub fn create_node(&mut self, type_name: &str, world_pos: egui::Vec2) -> Option<NodeId> {
        let def = self.registry.get(type_name)?.clone();
        self.edit("create");
        self.history.end_gesture();
        // New operators land in the network you are looking at.
        let parent = self.current;
        let id = self.graph.create(parent, &def, None).ok()?;
        self.graph.node_mut_quiet(id).pos = [world_pos.x, world_pos.y];
        otd_engine::panel_templates::install_if_panel(
            &mut self.graph,
            &self.registry,
            id,
            type_name,
        );
        self.select_only(id);
        Some(id)
    }


    /// Is a recording running right now?
    pub fn recording(&self) -> bool {
        self.recorder
            .filter(|id| self.graph.contains(*id))
            .map(|id| self.graph.node(id).params["record"].value == Value::Bool(true))
            .unwrap_or(false)
    }

    /// Record the viewer to a movie file, or stop the recording that is
    /// running.
    ///
    /// This is the Movie File Out operator, not a second recording path: the
    /// button makes the node, wires it to the viewer and flips its Record
    /// parameter. Everything a patch that was built by hand can do — codec,
    /// frame rate, quality — is still on the node, and the node survives in
    /// the saved project.
    pub fn toggle_record(&mut self) {
        if self.recording() {
            let id = self.recorder.unwrap();
            self.edit("record");
            let _ = self.graph.set_param(id, "record", Value::Bool(false));
            let file = self.graph.node(id).params["file"].value.as_str();
            self.status = format!("Recorded {file}");
            return;
        }
        let Some(comp) = self.perform_ui() else {
            self.status = "Nothing to record: designate a Perform UI container in Window Placement".into();
            return;
        };
        let Some(out_top) = self
            .graph
            .connector_ops(comp, otd_core::graph::Connector::Out)
            .first()
            .copied()
        else {
            self.status = "Perform COMP has no Out TOP to record".into();
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Movie", &["mp4", "mov"])
            .set_file_name("out.mp4")
            .save_file()
        else {
            return;
        };
        // Reuse the node across takes so the codec and frame rate you set on
        // the first recording still apply to the second.
        let id = match self.recorder.filter(|id| self.graph.contains(*id)) {
            Some(id) => id,
            None => {
                let pos = self.graph.node(comp).pos;
                let Some(id) =
                    self.create_node(otd_gpu::ops::MOVIE_OUT, egui::vec2(pos[0] + 180.0, pos[1]))
                else {
                    self.status = "Movie File Out is not registered".into();
                    return;
                };
                if let Err(e) = self.graph.connect(out_top, id, 0) {
                    self.status = format!("Could not wire the recorder: {e}");
                    return;
                }
                // Recording must not depend on the node being on screen, and
                // in perform mode nothing is.
                self.graph.node_mut(id).flags.render = true;
                self.recorder = Some(id);
                id
            }
        };
        self.edit("record");
        let _ = self
            .graph
            .set_param(id, "file", Value::Str(path.display().to_string()));
        let _ = self.graph.set_param(id, "record", Value::Bool(true));
        self.status = format!("Recording to {}", path.display());
    }

    // -------------------------------------------------------- project files

    /// Ask where to put it, always.
    ///
    /// Not `save(None)`, which is what Save As used to call: that falls back
    /// to the project's own path, so the one command whose entire job is to
    /// ask never asked. It silently wrote over the file you were trying to
    /// save a copy *of* — a Save As that behaves like Save is worse than one
    /// that does nothing, because you only find out later.
    pub fn save_as(&mut self) {
        // Seeded with the current name, the way every other application does
        // it: Save As is usually "the same thing, somewhere else" or "…with a
        // number on the end", and retyping the name is the annoying part.
        let suggested = self
            .project_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled.otd".to_string());
        let mut dialog = rfd::FileDialog::new()
            .add_filter("OpenTouchDesigner project", &["otd"])
            .set_file_name(&suggested);
        if let Some(dir) = self.project_path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.save_file() {
            self.save(Some(path));
        }
    }

    /// Write the project. `path` if given, the project's own path if it has
    /// one, and otherwise ask — an untitled patch has nowhere to go.
    pub fn save(&mut self, path: Option<PathBuf>) {
        let path = match path.or_else(|| self.project_path.clone()) {
            Some(p) => p,
            None => match rfd::FileDialog::new()
                .add_filter("OpenTouchDesigner project", &["otd"])
                .set_file_name("untitled.otd")
                .save_file()
            {
                Some(p) => p,
                None => return,
            },
        };
        let mut project = Project::from_graph(&self.graph, &self.registry, self.time.fps);
        project.settings = self.project_settings();
        match project.save(&path) {
            Ok(()) => {
                self.status = format!("Saved {}", path.display());
                self.project_path = Some(path);
                self.document_dirty = false;
            }
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    pub fn open(&mut self, path: Option<PathBuf>) {
        let path = match path.or_else(|| {
            rfd::FileDialog::new()
                .add_filter("OpenTouchDesigner project", &["otd"])
                .pick_file()
        }) {
            Some(p) => p,
            None => return,
        };
        match Project::load(&path) {
            Ok(project) => match project.to_graph(&self.registry) {
                Ok(graph) => {
                    self.assistant.reset_context();
                    self.graph = graph;
                    self.engines.top.reset();
                    self.cook.reset();
                    self.thumbs.clear();
                    self.history.clear();
                    self.clear_selection();
                    self.viewer = self
                        .graph
                        .walk()
                        .into_iter()
                        .rev()
                        .find(|&id| self.graph.node(id).family == otd_core::Family::Comp);
                    self.apply_project_settings(&project.settings);
                    otd_engine::panel_templates::upgrade_panel_internals(
                        &mut self.graph,
                        &self.registry,
                    );
                    self.current = self.graph.root();
                    self.view = CanvasView::default();
                    self.network_views.clear();
                    self.document_dirty = false;
                    self.status = format!("Opened {}", path.display());
                    self.project_path = Some(path);
                }
                Err(e) => self.status = format!("Open failed: {e}"),
            },
            Err(e) => self.status = format!("Open failed: {e}"),
        }
    }

    pub fn new_project(&mut self) {
        self.load_demo("starter");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Noise -> level -> out, plus an unrelated branch nobody wired up.
    fn patch() -> (Graph, OpRegistry, NodeId, NodeId) {
        let registry = otd_engine::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let noise = graph
            .create(root, registry.get("noiseTOP").unwrap(), Some("noise1"))
            .unwrap();
        let out = graph
            .create(root, registry.get("nullTOP").unwrap(), Some("out1"))
            .unwrap();
        graph.connect(noise, out, 0).unwrap();
        let spare = graph
            .create(root, registry.get("rampTOP").unwrap(), Some("ramp1"))
            .unwrap();
        (graph, registry, out, spare)
    }

    #[test]
    fn a_node_on_screen_cooks_and_the_same_node_off_screen_does_not() {
        let (graph, _reg, out, spare) = patch();

        // On the canvas: its own viewer makes it a root.
        let root = graph.root();
        let roots = ui::cook_roots(&graph, Some(out), &[spare], root);
        assert!(roots.contains(&spare));

        // Panned away: nothing wants it, so nothing cooks it.
        let roots = ui::cook_roots(&graph, Some(out), &[], root);
        assert!(!roots.contains(&spare));
        assert!(roots.contains(&out), "perform output chain stays a root");
    }

    #[test]
    fn perform_mode_leaves_only_the_output_chain() {
        let (mut graph, _reg, out, spare) = patch();
        // A render-flagged node is a root wherever the canvas is looking —
        // that is what the flag means, and a show output relies on it.
        graph.node_mut(spare).flags.render = true;

        // Perform mode is exactly "nothing is on the canvas".
        let performing = ui::cook_roots(&graph, Some(out), &[], graph.root());
        assert!(performing.contains(&out));
        assert!(
            performing.contains(&spare),
            "the render flag survives perform mode"
        );

        let noise = graph.find("/noise1").unwrap();
        assert!(
            !performing.contains(&noise),
            "an upstream node is pulled by its consumer, not listed as a root"
        );
    }

    #[test]
    fn a_deleted_viewer_does_not_become_a_root() {
        let (mut graph, _reg, out, _) = patch();
        graph.remove(out).unwrap();
        // A stale NodeId from a deleted node must not reach the cook.
        assert!(ui::cook_roots(&graph, Some(out), &[], graph.root()).is_empty());
    }

    #[test]
    fn cook_roots_tolerates_stale_current_network_id() {
        let graph = Graph::new();
        let roots = ui::cook_roots(&graph, None, &[], NodeId::default());
        assert!(roots.is_empty());
    }
}
