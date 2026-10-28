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

use crate::canvas::{CanvasView, DragState};

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
    /// Where the network editor was drawn last frame. Drag-and-drop needs it
    /// to turn a pointer position into a place in the network.
    pub canvas_rect: egui::Rect,
    pub drag: Option<DragState>,
    pub create_dialog: Option<CreateDialog>,
    /// Screen rect of the Add operator window, for blocking canvas zoom while
    /// the pointer is over it.
    pub create_dialog_rect: Option<egui::Rect>,
    pub show_perf: bool,
    /// The performance monitor window.
    pub show_monitor: bool,
    pub show_textport: bool,
    pub textport: crate::textport::Textport,
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
    pub open_container_views: BTreeSet<NodeId>,

    /// Mouse and keyboard, sampled each frame for the input CHOPs.
    pub input_state: otd_chop::InputState,
    /// Draft state for the "add a component parameter" row.
    pub new_param_name: String,
    pub new_param_type: String,
    /// Active inspector tab per node (TouchDesigner-style pages).
    pub param_inspector_page: HashMap<NodeId, String>,

    thumbs: HashMap<NodeId, (TextureId, u64)>,
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
    blur_params_on_select: bool,
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

pub struct CreateDialog {
    pub filter: String,
    pub family: otd_core::Family,
    /// Where the new node lands, in canvas space.
    pub world_pos: egui::Vec2,
    pub focus: bool,
    /// Wire this node's output into the new node's first input, if types allow.
    pub connect_from: Option<NodeId>,
    pub wire_drop: Option<WireDrop>,
}

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

        let mut app = OtdApp {
            graph: Graph::new(),
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
            current: NodeId::default(),
            view: CanvasView::default(),
            canvas_rect: egui::Rect::NOTHING,
            drag: None,
            create_dialog: None,
            create_dialog_rect: None,
            show_perf: true,
            show_monitor: false,
            show_textport: false,
            textport: crate::textport::Textport::default(),
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
            open_container_views: BTreeSet::new(),
            input_state: otd_chop::InputState::default(),
            new_param_name: String::new(),
            new_param_type: "float".to_string(),
            param_inspector_page: HashMap::new(),
            thumbs: HashMap::new(),
            visible: Vec::new(),
            history: History::default(),
            assistant: crate::assistant::Assistant::default(),
            project_path: None,
            status: String::new(),
            cook_error: None,
            channel_expr_clipboard: String::new(),
            node_clipboard: NodeClipboard::default(),
            paste_count: 0,
            blur_params_on_select: false,
        };
        // Not `starter`: that patch is the Phase 0 exit criterion, and an
        // animated grey noise field is a bad first argument for what this
        // tool is. `tunnel` is nine nodes, no shader, and looks like the
        // thing somebody opened the program hoping to make.
        app.load_demo("tunnel");
        app.textport
            .info(0, "Textport — cook errors, callbacks, and Python (Enter to run).");
        if let Some(e) = app.engines.python_error() {
            app.textport.error(0, e);
        }
        Ok(app)
    }

    /// Load one of the built-in patches (see `otd_gpu::demo`). The editor
    /// opens on `starter`, which is the Phase 0 exit criterion made visible.
    pub fn load_demo(&mut self, name: &str) {
        let Some((graph, _out)) = otd_engine::demo::by_name(name, &self.registry) else {
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
        self.time = CookContext::default();
        self.current = self.graph.root();
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
            self.enter(comp);
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

    fn cook_roots(&self) -> Vec<NodeId> {
        let perform_ui = self.perform_ui();
        let mut roots = cook_roots(&self.graph, perform_ui, &self.visible, self.current);
        crate::window_comp::append_cook_roots(&self.graph, &mut roots);
        if let Some(top) = crate::window_comp::perform_display_top(
            &self.graph,
            self.main_perform_window,
            perform_ui,
        ) {
            if !roots.contains(&top) {
                roots.push(top);
            }
        }
        if let Some(comp) = perform_ui {
            crate::canvas::comp_display::append_perform_panel_cook_roots(
                &self.graph,
                comp,
                &mut roots,
            );
        }
        crate::container_view::append_cook_roots(&self.graph, &self.open_container_views, &mut roots);
        roots
    }

    fn cook_frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_instant).as_secs_f64().min(0.25);
        self.last_instant = now;
        self.smoothed_fps = self.smoothed_fps * 0.9 + (1.0 / dt.max(1e-6)) * 0.1;

        if self.playing && !self.scrubbing {
            self.time.advance(dt);
            let (start, end) = self.loop_range;
            if self.looping && end > start && self.time.time >= end {
                // Wrap by the loop length rather than snapping to the start:
                // the leftover carries into the next pass, so a loop shorter
                // than a frame interval does not drift or stall.
                let length = end - start;
                self.time.time = start + (self.time.time - start).rem_euclid(length);
            }
        }
        self.scrubbing = false;

        // Clones follow their master. An unchanged master costs one subtree
        // walk, so this is cheap enough to do every frame and means the
        // editor never shows a stale copy.
        // Replicators first — they may create clones for syncing to fill.
        let replicated = otd_engine::replicator::sync(&mut self.graph, &self.engines.dats);
        let synced = self.graph.sync_clones(&self.registry);
        if replicated > 0 {
            self.status = format!("replicated {replicated} node(s)");
        } else if synced > 0 {
            self.status = format!("synced {synced} clone(s)");
        }

        let roots = self.cook_roots();
        self.engines.set_input_state(self.input_state.clone());
        self.engines.begin_frame();
        let result =
            self.cook
                .cook_frame(&self.graph, &roots, &self.time.clone(), &mut self.engines);
        self.engines.end_frame();

        self.cook_error = result.err().map(|e| e.to_string());
        let frame = self.time.frame;
        if let Some(err) = &self.cook_error {
            if self.textport.last_cook_error.as_deref() != Some(err.as_str()) {
                self.textport.error(frame, err.clone());
                self.textport.last_cook_error = Some(err.clone());
            }
        } else {
            self.textport.last_cook_error = None;
        }

        // Between frames, where the graph is held mutably, is the only place a
        // callback's parameter change can land. See `otd_core::edit`.
        let edits = self.engines.take_edits();
        if !edits.is_empty() {
            let (applied, problems) = otd_core::edit::apply(&mut self.graph, &edits);
            for p in &problems {
                self.textport.warn(frame, p);
            }
            if let Some(first) = problems.first() {
                self.status = format!("callback: {first}");
            } else if applied > 0 {
                self.status = format!("callback set {applied} parameter(s)");
                self.textport
                    .info(frame, format!("callback applied {applied} parameter edit(s)"));
            }
        }
        for line in self.engines.take_script_log() {
            self.textport.info(frame, line);
        }

        let ms = self.cook.stats.total_cook_us as f64 / 1000.0;
        self.smoothed_cook_ms = self.smoothed_cook_ms * 0.9 + ms * 0.1;
    }

    /// The egui texture handle for a node's current output, registering or
    /// refreshing it only when the underlying texture object changed.
    pub fn set_perform_ui(&mut self, id: NodeId) {
        if !self.graph.contains(id) {
            return;
        }
        if self.graph.node(id).family != otd_core::Family::Comp {
            self.status = "Perform UI must be a COMP".into();
            return;
        }
        self.edit("perform UI");
        self.history.end_gesture();
        self.main_perform_ui_path = self.graph.path(id);
        self.viewer = Some(id);
        self.enter(id);
        self.select_only(id);
    }

    pub fn toggle_perform_ui(&mut self, id: NodeId) {
        if self.perform_ui() == Some(id) {
            self.edit("perform UI");
            self.history.end_gesture();
            self.main_perform_ui_path.clear();
            self.viewer = None;
        } else {
            self.set_perform_ui(id);
        }
    }

    pub fn perform_output_thumbnail(&mut self) -> Option<(TextureId, [u32; 2])> {
        let out = crate::window_comp::perform_display_top(
            &self.graph,
            self.main_perform_window,
            self.perform_ui(),
        )?;
        self.thumbnail(out)
    }

    pub fn thumbnail(&mut self, id: NodeId) -> Option<(TextureId, [u32; 2])> {
        let tex = self.engines.top.output(&self.graph, id)?.clone();
        let size = [tex.key.width, tex.key.height];
        let entry = self.thumbs.get(&id).copied();
        match entry {
            Some((tid, seen)) if seen == tex.generation => Some((tid, size)),
            Some((tid, _)) => {
                self.render_state
                    .renderer
                    .write()
                    .update_egui_texture_from_wgpu_texture(
                        &self.render_state.device,
                        &tex.view,
                        wgpu::FilterMode::Linear,
                        tid,
                    );
                self.thumbs.insert(id, (tid, tex.generation));
                Some((tid, size))
            }
            None => {
                let tid = self.render_state.renderer.write().register_native_texture(
                    &self.render_state.device,
                    &tex.view,
                    wgpu::FilterMode::Linear,
                );
                self.thumbs.insert(id, (tid, tex.generation));
                Some((tid, size))
            }
        }
    }

    /// Write the selected component out as a `.otdc`.
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
        self.blur_params_on_select = true;
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
            self.delete_selected();
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

    pub fn paste_clipboard(&mut self, origin: [f32; 2]) {
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

    /// Step inside a component. Anything else is ignored, so a stray
    /// double-click on an operator does not lose your place.
    pub fn enter(&mut self, id: NodeId) {
        if self.graph.get(id).map(|n| n.family) == Some(otd_core::Family::Comp) {
            self.current = id;
            self.clear_selection();
            self.view = CanvasView::default();
        }
    }

    /// Step out to the parent network.
    pub fn leave(&mut self) {
        if let Some(parent) = self.graph.get(self.current).and_then(|n| n.parent) {
            let was = self.current;
            self.current = parent;
            self.select_only(was);
            self.view = CanvasView::default();
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
        }
        self.visible.retain(|id| self.graph.contains(*id));
        self.drag = None;
    }

    // ---------------------------------------------------------- graph edits

    /// Delete everything selected, as one undo step.
    pub fn delete_selected(&mut self) {
        if let Some((dst, input)) = self.selected_wire {
            if self.graph.contains(dst) && !self.graph.node(dst).flags.lock {
                self.edit("unwire");
                self.history.end_gesture();
                let _ = self.graph.disconnect(dst, input);
                self.selected_wire = None;
                self.status.clear();
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
        for id in &live {
            self.engines.top.forget(*id);
            self.thumbs.remove(id);
            self.cook.forget(*id);
            let _ = self.graph.remove(*id);
            if self.perform_ui() == Some(*id) {
                self.main_perform_ui_path.clear();
                self.viewer = None;
            }
        }
        for (src, consumer, slot) in splices {
            let _ = self.graph.connect(src, consumer, slot);
        }
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

impl eframe::App for OtdApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Cook before drawing: the thumbnails egui is about to sample are the
        // textures we just rendered into.
        self.cook_frame();

        if self.blur_params_on_select {
            self.blur_params_on_select = false;
            crate::canvas::input::release_param_focus(ui.ctx());
        }
        self.clipboard_keys(ui.ctx());

        self.create_operator_tab_key(ui.ctx());
        self.history_keys(ui.ctx());
        self.file_keys(ui.ctx());
        self.selection_keys(ui.ctx());
        self.flag_keys(ui.ctx());

        // F1 anywhere, including out of perform mode — a performer who cannot
        // find the way back out of a black screen has a real problem.
        if ui.ctx().input(|i| i.key_pressed(egui::Key::F1)) {
            if self.perform_preview_only() {
                self.leave_perform_for_edit();
            } else if self.perform && self.perform_edit {
                self.perform_edit = false;
            } else {
                self.perform = true;
                self.perform_edit = false;
            }
        }
        if self.perform_preview_only()
            && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::E))
        {
            self.perform_edit = true;
            self.focus_perform_ui_for_edit();
        }
        if self.perform && self.perform_edit && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.leave_perform_for_edit();
        }
        let want_fullscreen = self.perform_preview_only();
        if want_fullscreen != self.perform_applied {
            self.perform_applied = want_fullscreen;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Fullscreen(want_fullscreen));
        }
        // F2 records, in either mode — the top bar is not there in the one
        // where you most want to hit record.
        if ui.ctx().input(|i| i.key_pressed(egui::Key::F2)) {
            self.toggle_record();
        }
        if ui.ctx().input(|i| i.modifiers.alt && i.key_pressed(egui::Key::T)) {
            self.show_textport = !self.show_textport;
        }
        if ui.ctx().input(|i| i.modifiers.alt && i.key_pressed(egui::Key::W)) {
            self.show_window_placement = !self.show_window_placement;
        }
        if self.designer_position_pending {
            self.designer_position_pending = false;
            self.apply_designer_window_position(ui.ctx());
        }

        if self.perform_preview_only() {
            self.perform_view(ui);
            self.perform_preview_hint(ui);
        } else {
            if self.perform && self.perform_edit {
                self.perform_edit_banner(ui);
            }
            self.top_bar(ui);
            self.monitor(ui.ctx());
            crate::textport::Textport::show(self, ui.ctx());
            self.timeline(ui);
            self.side_panel(ui);
            crate::canvas::show(self, ui);
            self.clipboard_keys(ui.ctx());
        }
        crate::assistant::bar(self, ui.ctx());
        crate::assistant::window(self, ui.ctx());
        if !self.perform_preview_only() {
            crate::create_dialog::show(self, ui);
        }
        self.output_viewport(ui.ctx());
        self.window_comp_viewports(ui.ctx());
        if !self.perform_preview_only() {
            crate::container_view::show(self, ui.ctx());
        }
        crate::window_placement::show(self, ui.ctx());

        // Files can be dropped in either mode: a performer swapping a clip
        // mid-show should not have to leave the output to do it.
        crate::media::hover_overlay(self, ui.ctx());
        crate::media::handle_drops(self, ui.ctx());

        // A realtime tool repaints continuously; there is always time moving.
        ui.ctx().request_repaint();
    }
}

impl OtdApp {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Edit", |ui| {
                    let (back, ahead) = self.history.depth();
                    let has_sel = !self.selection.is_empty() || self.selected.is_some();
                    let can_paste = !self.node_clipboard.nodes.is_empty();
                    if ui
                        .add_enabled(back > 0, egui::Button::new("Undo").shortcut_text("Cmd+Z"))
                        .clicked()
                    {
                        self.undo();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            ahead > 0,
                            egui::Button::new("Redo").shortcut_text("Cmd+Shift+Z"),
                        )
                        .clicked()
                    {
                        self.redo();
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(has_sel, egui::Button::new("Copy").shortcut_text("Cmd+C"))
                        .clicked()
                    {
                        self.copy_selection(ui.ctx());
                        ui.close();
                    }
                    if ui
                        .add_enabled(has_sel, egui::Button::new("Cut").shortcut_text("Cmd+X"))
                        .clicked()
                    {
                        self.cut_selection(ui.ctx());
                        ui.close();
                    }
                    if ui
                        .add_enabled(can_paste, egui::Button::new("Paste").shortcut_text("Cmd+V"))
                        .clicked()
                    {
                        let ctx = ui.ctx();
                        let pointer = ctx
                            .pointer_latest_pos()
                            .unwrap_or_else(|| self.canvas_rect.center());
                        let world = self.view.to_world(self.canvas_rect.min, pointer);
                        self.paste_clipboard([world.x, world.y]);
                        ui.close();
                    }
                });
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_project();
                        ui.close();
                    }
                    if ui.button("Open…").clicked() {
                        self.open(None);
                        ui.close();
                    }
                    if ui
                        .add(egui::Button::new("Save").shortcut_text("Cmd+S"))
                        .clicked()
                    {
                        let p = self.project_path.clone();
                        self.save(p);
                        ui.close();
                    }
                    if ui
                        .add(egui::Button::new("Save As…").shortcut_text("Cmd+Shift+S"))
                        .clicked()
                    {
                        self.save_as();
                        ui.close();
                    }
                    if ui
                        .button("Export Bundle…")
                        .on_hover_text(
                            "Copy the project and every component it uses into one folder, \
                             ready to move to a show machine",
                        )
                        .clicked()
                    {
                        self.export_bundle();
                        ui.close();
                    }
                    ui.separator();
                    let on_comp = self
                        .selected
                        .and_then(|s| self.graph.get(s))
                        .map(|n| n.family == otd_core::Family::Comp)
                        .unwrap_or(false);
                    if ui
                        .add_enabled(on_comp, egui::Button::new("Save Component As…"))
                        .on_hover_text("Write the selected component to a .otdc file")
                        .clicked()
                    {
                        self.save_component();
                        ui.close();
                    }
                    if ui
                        .button("Import Component…")
                        .on_hover_text("Load a .otdc into this network, linked to its file")
                        .clicked()
                    {
                        self.import_component();
                        ui.close();
                    }
                    ui.separator();
                    ui.menu_button("Examples", |ui| {
                        for name in otd_engine::demo::NAMES {
                            if ui.button(*name).clicked() {
                                self.load_demo(name);
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Palette", |ui| {
                        for item in otd_engine::palette::ITEMS {
                            if ui.button(item.name).on_hover_text(item.summary).clicked() {
                                self.edit("palette");
                                let id = item.build(&mut self.graph, &self.registry, self.current);
                                self.select_only(id);
                                self.status = format!("added {} from the palette", item.name);
                                ui.close();
                            }
                        }
                    });
                });
                self.media_menu(ui);
                ui.menu_button("Dialogs", |ui| {
                    if ui
                        .add(egui::Button::new("Window Placement…").shortcut_text("Alt+W"))
                        .clicked()
                    {
                        self.show_window_placement = true;
                        ui.close();
                    }
                });

                // Undo is on the bar, not only in the menu and the shortcut.
                // It is the one thing reached for in a hurry and while looking
                // at the network rather than at the menu, and a step you
                // cannot see how to take is a step you do not take. Disabled
                // when there is nothing to undo, so the bar also answers
                // "is there anything to go back to".
                ui.separator();
                let (back, ahead) = self.history.depth();
                let undo = ui.push_id(egui::Id::new("otd_topbar_undo"), |ui| {
                    ui.add_enabled(back > 0, egui::Button::new("↶"))
                });
                if undo.inner.on_hover_text("Undo (Cmd+Z)").clicked() {
                    self.undo();
                }
                let redo = ui.push_id(egui::Id::new("otd_topbar_redo"), |ui| {
                    ui.add_enabled(ahead > 0, egui::Button::new("↷"))
                });
                if redo.inner.on_hover_text("Redo (Cmd+Shift+Z)").clicked() {
                    self.redo();
                }

                ui.separator();
                let icon = if self.playing { "⏸" } else { "▶" };
                if ui.button(icon).on_hover_text("Play / pause time").clicked() {
                    self.playing = !self.playing;
                }
                if ui.button("⟲").on_hover_text("Reset time to 0").clicked() {
                    self.time.frame = 0;
                    self.time.time = 0.0;
                    self.time.abs_time = 0.0;
                }
                ui.label(format!(
                    "frame {}   {:.2}s",
                    self.time.frame, self.time.abs_time
                ));

                ui.separator();
                // Breadcrumb: click a step to go back to it.
                let mut chain = Vec::new();
                let mut cur = Some(self.current);
                while let Some(c) = cur {
                    chain.push(c);
                    cur = self.graph.get(c).and_then(|n| n.parent);
                }
                chain.reverse();
                for (i, id) in chain.iter().enumerate() {
                    if i > 0 {
                        ui.label(egui::RichText::new("/").weak());
                    }
                    let name = if *id == self.graph.root() {
                        "/".to_string()
                    } else {
                        self.graph.node(*id).name.clone()
                    };
                    let last = i + 1 == chain.len();
                    let text = if last {
                        egui::RichText::new(name).strong()
                    } else {
                        egui::RichText::new(name).weak()
                    };
                    if ui.selectable_label(false, text).clicked() {
                        self.current = *id;
                        self.clear_selection();
                    }
                }

                ui.separator();
                if ui
                    .button("Perform")
                    .on_hover_text("Hide the editor and show only the output (F1)")
                    .clicked()
                {
                    self.perform = true;
                    self.perform_edit = false;
                }
                let recording = self.recording();
                let label = if recording { "⏹ Stop" } else { "⏺ Record" };
                if ui
                    .button(label)
                    .on_hover_text("Record the viewer to a movie file (F2)")
                    .clicked()
                {
                    self.toggle_record();
                }
                ui.toggle_value(&mut self.assistant.bar, "✨ Assistant")
                    .on_hover_text("Describe a patch and have it built here — Cmd/Ctrl+K");
                ui.toggle_value(&mut self.show_monitor, "Perf")
                    .on_hover_text("Per-node cook cost and GPU memory");
                ui.toggle_value(&mut self.show_textport, "Textport")
                    .on_hover_text("Log and Python console (Alt+T)");
                ui.toggle_value(&mut self.output_window, "Output")
                    .on_hover_text("Open a second window showing only the viewer");
                if self.output_window {
                    ui.toggle_value(&mut self.output_fullscreen, "Fullscreen");
                }

                ui.separator();
                ui.label(format!("{:.0} fps", self.smoothed_fps));
                ui.label(format!(
                    "cook {:.2} ms  ({} cooked / {} cached)",
                    self.smoothed_cook_ms, self.cook.stats.cooked, self.cook.stats.cached
                ));
                ui.label(format!(
                    "{} passes  {:.0} MB",
                    self.engines.top.passes_this_frame,
                    (self.engines.top.resident_bytes() + self.engines.top.pooled_bytes()) as f64
                        / 1.0e6
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (text, color) = match &self.cook_error {
                        Some(err) => (
                            err.as_str(),
                            egui::Color32::from_rgb(230, 110, 110),
                        ),
                        None => (&self.status as &str, ui.style().visuals.text_color()),
                    };
                    ui.push_id(egui::Id::new("otd_status"), |ui| {
                        ui.label(egui::RichText::new(text).color(color));
                    });
                });
            });
        });
    }

    /// Your own material, and the things to do to it.
    ///
    /// Everything here is reachable another way — Tab finds any operator, and
    /// a file parameter has a Browse button. It is a menu because the other
    /// ways all require knowing the name of an operator first.
    fn media_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Media", |ui| {
            if ui
                .button("Import Media…")
                .on_hover_text("Movies, images and audio — or just drop the files on the network")
                .clicked()
            {
                crate::media::import_dialog(self);
                ui.close();
            }
            if ui
                .button("Use Webcam")
                .on_hover_text("A Video Device In, viewing straight away")
                .clicked()
            {
                crate::media::add_webcam(self);
                ui.close();
            }
            ui.separator();

            let selected = self
                .selected
                .filter(|s| self.graph.contains(*s))
                .map(|s| (self.graph.node(s).name.clone(), self.graph.node(s).family));
            let label = match &selected {
                Some((name, family)) => format!("Add {} after {name}", family.suffix()),
                None => "Add operator".to_string(),
            };
            let only_family = selected.as_ref().map(|(_, f)| *f);
            ui.add_enabled_ui(selected.is_some(), |ui| {
                ui.menu_button(label, |ui| {
                    crate::media::add_operator_menu(self, ui, only_family);
                });
            });
            if selected.is_none() {
                ui.label(
                    egui::RichText::new("select an operator to add downstream")
                        .weak()
                        .small(),
                );
            }
        });
    }

    /// The performance monitor: what each node costs, and what the GPU holds.
    ///
    /// Ranked by cost *per frame* rather than by cook time. Those are not the
    /// same number in a demand-driven engine, and ranking by the wrong one
    /// sends you off optimising a node that cooks once and then sits there.
    fn monitor(&mut self, ctx: &egui::Context) {
        if !self.show_monitor {
            return;
        }
        let mut open = self.show_monitor;
        egui::Window::new("Performance")
            .open(&mut open)
            .default_width(420.0)
            .default_height(360.0)
            .show(ctx, |ui| {
                let budget = 1000.0 / self.time.fps.max(1.0);
                let frame_ms = 1000.0 / self.smoothed_fps.max(1e-6);
                ui.horizontal(|ui| {
                    ui.strong(format!("{frame_ms:.2} ms/frame"));
                    ui.label(
                        egui::RichText::new(format!("budget {budget:.2} ms"))
                            .weak()
                            .small(),
                    );
                    let (colour, verdict) = if frame_ms <= budget {
                        (egui::Color32::from_rgb(130, 200, 140), "holding the rate")
                    } else {
                        (egui::Color32::from_rgb(235, 120, 120), "over budget")
                    };
                    ui.colored_label(colour, egui::RichText::new(verdict).small());
                });
                ui.label(
                    egui::RichText::new(format!(
                        "cook {:.2} ms · {} cooked, {} cached last frame",
                        self.smoothed_cook_ms, self.cook.stats.cooked, self.cook.stats.cached
                    ))
                    .weak()
                    .small(),
                );

                let top = self.engines.top.pooled_bytes();
                let resident = self.engines.top.resident_bytes();
                ui.label(
                    egui::RichText::new(format!(
                        "GPU  {} in node outputs · {} pooled · {} textures created",
                        bytes(resident),
                        bytes(top),
                        self.engines.top.textures_created()
                    ))
                    .weak()
                    .small(),
                );
                ui.separator();

                // Every node that has cooked, most expensive per frame first.
                let mut rows: Vec<(NodeId, f64, f64, f64)> = self
                    .graph
                    .walk()
                    .into_iter()
                    .filter(|id| *id != self.graph.root())
                    .map(|id| {
                        (
                            id,
                            self.cook.frame_cost_ms(id),
                            self.cook.avg_cook_ms(id),
                            self.cook.cook_rate(id),
                        )
                    })
                    .filter(|(_, _, avg, _)| *avg > 0.0)
                    .collect();
                rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                if rows.is_empty() {
                    ui.label(egui::RichText::new("nothing has cooked yet").weak());
                    return;
                }
                let worst = rows[0].1.max(1e-6);

                let mut jump = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("perf-rows")
                        .num_columns(4)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("node").small().strong());
                            ui.label(egui::RichText::new("ms/frame").small().strong());
                            ui.label(egui::RichText::new("per cook").small().strong());
                            ui.label(egui::RichText::new("cooks").small().strong());
                            ui.end_row();

                            for (id, cost, avg, rate) in &rows {
                                let path = self.graph.path(*id);
                                if ui.selectable_label(false, &path).clicked() {
                                    jump = Some(*id);
                                }
                                // Tint by share of the worst offender, so the
                                // eye lands on the problem without reading.
                                let heat = (cost / worst) as f32;
                                ui.colored_label(
                                    egui::Color32::from_rgb(
                                        (150.0 + 85.0 * heat) as u8,
                                        (200.0 - 80.0 * heat) as u8,
                                        (160.0 - 60.0 * heat) as u8,
                                    ),
                                    egui::RichText::new(format!("{cost:.3}")).monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{avg:.3}")).monospace().weak(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.0}%", rate * 100.0))
                                        .monospace()
                                        .weak(),
                                );
                                ui.end_row();
                            }
                        });
                });
                // Clicking a row selects the node, so the monitor is a way in
                // rather than a wall of numbers to go hunting from.
                if let Some(id) = jump {
                    self.select_only(id);
                }
            });
        self.show_monitor = open;
    }

    /// The timeline strip: a scrubbable playhead and the loop range.
    ///
    /// The playhead is the same `CookContext` the whole network reads, so
    /// dragging it drags the entire patch — every time-dependent operator, the
    /// keyframe curves, the LFOs. There is no separate "timeline time" that
    /// could get out of step with what is rendering.
    fn timeline(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("timeline")
            .exact_size(52.0)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let icon = if self.playing { "⏸" } else { "▶" };
                    if ui
                        .button(icon)
                        .on_hover_text("Play / pause (Space)")
                        .clicked()
                    {
                        self.playing = !self.playing;
                    }
                    if ui
                        .button("⏮")
                        .on_hover_text("Back to the loop start")
                        .clicked()
                    {
                        self.time.time = self.loop_range.0;
                        self.time.frame = (self.loop_range.0 * self.time.fps).round() as i64;
                    }
                    ui.toggle_value(&mut self.looping, "Loop");

                    ui.label("from");
                    ui.add(
                        egui::DragValue::new(&mut self.loop_range.0)
                            .speed(0.05)
                            .range(0.0..=3600.0)
                            .suffix("s"),
                    );
                    ui.label("to");
                    ui.add(
                        egui::DragValue::new(&mut self.loop_range.1)
                            .speed(0.05)
                            .range(0.0..=3600.0)
                            .suffix("s"),
                    );
                    // An inverted range would make the wrap arithmetic
                    // meaningless; keep the end after the start rather than
                    // guarding at every use.
                    if self.loop_range.1 <= self.loop_range.0 {
                        self.loop_range.1 = self.loop_range.0 + 0.1;
                    }

                    ui.label(
                        egui::RichText::new(format!(
                            "frame {}    {:.2}s",
                            self.time.frame, self.time.time
                        ))
                        .monospace(),
                    );
                });
                self.playhead(ui);
            });
    }

    /// The scrub bar. Click or drag anywhere on it to move time.
    fn playhead(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 16.0),
            egui::Sense::click_and_drag(),
        );
        let (start, end) = self.loop_range;
        let span = (end - start).max(1e-6);

        let painter = ui.painter();
        painter.rect_filled(rect, 3.0, egui::Color32::from_gray(30));

        // Second ticks, as long as they are not so dense as to be a smear.
        let seconds = span.ceil() as i64;
        if seconds <= 120 {
            for i in 0..=seconds {
                let t = start.floor() + i as f64;
                if t < start || t > end {
                    continue;
                }
                let x = rect.left() + ((t - start) / span) as f32 * rect.width();
                painter.line_segment(
                    [
                        egui::pos2(x, rect.bottom() - 5.0),
                        egui::pos2(x, rect.bottom()),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
                );
            }
        }

        if resp.dragged() || resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let u = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
                self.time.time = start + u * span;
                self.time.frame = (self.time.time * self.time.fps).round() as i64;
                // Scrubbing owns time for this frame: letting the transport
                // also advance would fight the drag and jitter the playhead.
                self.scrubbing = true;
            }
        }

        let u = ((self.time.time - start) / span).clamp(0.0, 1.0) as f32;
        let x = rect.left() + u * rect.width();
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(230, 170, 90)),
        );
    }

    fn perform_preview_hint(&self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let pad = egui::vec2(12.0, 10.0);
        let pos = rect.left_bottom() + pad;
        let text = "Esc or E — edit Perform UI    F1 — preview";
        ui.painter().text(
            pos,
            egui::Align2::LEFT_BOTTOM,
            text,
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgba_unmultiplied(200, 204, 212, 180),
        );
    }

    fn perform_edit_banner(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("perform_edit_banner").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Perform UI edit")
                        .strong()
                        .color(egui::Color32::from_rgb(220, 200, 140)),
                );
                if ui.button("Preview (F1)").clicked() {
                    self.perform_edit = false;
                }
                if ui.button("Exit perform").clicked() {
                    self.leave_perform_for_edit();
                }
            });
        });
    }

    /// Perform preview: fullscreen **View** of the perform-ui containerCOMP.
    ///
    /// Same path as `view_panel_comp`: recursive container layout, leaf panel
    /// TOPs, arbitrary TOPs only via opviewerCOMP.
    fn perform_view(&mut self, ui: &mut egui::Ui) {
        let perform_comp = self.perform_ui();
        let widgets = perform_comp
            .map(|c| otd_engine::panel::panel_display_widgets(&self.graph, c))
            .unwrap_or_default();
        let mut panels: Vec<(NodeId, f64)> = Vec::new();
        self.visible.clear();

        egui::CentralPanel::no_frame()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ui, |ui| {
                let area = ui.max_rect();
                match perform_comp {
                    Some(comp) if !widgets.is_empty() => {
                        panels.extend(draw_panel_comp(
                            ui, self, comp, area, &widgets, true,
                        ));
                    }
                    Some(comp) => {
                        if let Some(top) =
                            crate::canvas::comp_display::comp_view_top(&self.graph, comp)
                        {
                            if let Some((tid, size)) = self.thumbnail(top) {
                                let lb = letterbox(ui, tid, size);
                                draw_panel_comp_look(&self.graph, comp, ui.painter(), lb);
                                let w = otd_engine::panel::panel_display_widgets(&self.graph, comp);
                                panels.extend(draw_panel(ui, &w, lb, Some(self), true));
                            }
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    egui::RichText::new("Panel has no cooked view — open the COMP and wire a view TOP")
                                        .color(egui::Color32::from_gray(90)),
                                );
                            });
                        }
                    }
                    None => {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new("no Perform UI container — set one in Window Placement")
                                    .color(egui::Color32::from_gray(90)),
                            );
                        });
                    }
                }
            });

        // Applied after the panel is drawn, because the write needs the graph
        // mutably and drawing borrowed it.
        for (id, value) in panels {
            self.edit("panel");
            let _ = self
                .graph
                .set_param(id, "value", otd_core::Value::Float(value));
        }
    }

    /// The output window: a second OS window showing only the viewer TOP,
    /// which is what gets dragged onto a projector (PLAN.md Phase 1).
    fn output_viewport(&mut self, ctx: &egui::Context) {
        if self.output_closed.swap(false, Ordering::Relaxed) {
            self.output_window = false;
        }
        if !self.output_window {
            return;
        }
        let tex = self.perform_output_thumbnail();
        let closed = self.output_closed.clone();
        let builder = egui::ViewportBuilder::default()
            .with_title("OpenTouchDesigner — Output")
            .with_inner_size([1280.0, 720.0])
            .with_fullscreen(self.output_fullscreen);

        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("otd-output"),
            builder,
            move |ui, _class| {
                egui::CentralPanel::no_frame()
                    .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
                    .show(ui, |ui| {
                        if let Some((tid, size)) = tex {
                            let lb = letterbox(ui, tid, size);
                            draw_panel_texture(ui.painter(), lb, tid, size);
                        }
                    });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    closed.store(true, Ordering::Relaxed);
                }
            },
        );
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("params")
            .default_size(400.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    crate::params::show(self, ui);
                });
            });
    }
}

/// What has to cook this frame: perform output, render/cook roots, execute
/// DATs, and every node whose viewer flag is on and on screen.
///
/// That last condition is what makes the demand-driven engine visible: pan a
/// heavy branch off the canvas and it stops costing anything. It is also what
/// makes perform mode cheap rather than merely dark — nothing is on the canvas
/// there, so `visible` is empty and the only thing left is the output chain.
fn cook_roots(
    graph: &Graph,
    viewer: Option<NodeId>,
    visible: &[NodeId],
    current: NodeId,
) -> Vec<NodeId> {
    let mut roots = Vec::new();
    if let Some(v) = viewer.filter(|v| graph.contains(*v)) {
        roots.push(v);
    }
    crate::canvas::comp_display::append_network_display_roots(graph, current, visible, &mut roots);
    for id in graph.walk() {
        if id == graph.root() {
            continue;
        }
        let node = graph.node(id);
        let wanted = node.flags.render
            || node.flags.cook
            || otd_engine::execute::is_execute(&node.op_type)
            || (node.flags.viewer && visible.contains(&id) && node.family != otd_core::Family::Comp);
        if wanted && !roots.contains(&id) {
            roots.push(id);
        }
    }
    roots
}

/// Bytes, in whatever unit keeps the number short.
fn bytes(n: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if n as f64 >= MB {
        format!("{:.1} MB", n as f64 / MB)
    } else {
        format!("{:.0} kB", n as f64 / 1024.0)
    }
}

/// Paint a texture centred and letterboxed.
///
/// Letterbox rather than stretch, in both the output window and perform mode:
/// a show output at the wrong aspect ratio is worse than black bars, and worse
/// still because it looks deliberate.
/// Returns the rect it painted into, which is the box a panel lays out
/// against — the picture, not the window, so a widget stays where it was put
/// relative to the image whatever shape the window is.
pub(crate) fn letterbox(ui: &egui::Ui, tid: TextureId, size: [u32; 2]) -> egui::Rect {
    let avail = ui.available_size();
    let aspect = size[0] as f32 / size[1].max(1) as f32;
    let (mut w, mut h) = (avail.x, avail.x / aspect);
    if h > avail.y {
        h = avail.y;
        w = h * aspect;
    }
    let rect = egui::Rect::from_center_size(ui.max_rect().center(), egui::vec2(w, h));
    ui.painter().image(
        tid,
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    rect
}

/// Draw the panel widgets over `area`, returning the values a person changed.
///
/// The edits come back rather than being applied here: writing needs the graph
/// mutably, and the caller is already holding it to have drawn at all. Keeping
/// that split also means this function is only about pixels.
fn panel_widget_rect(area: egui::Rect, w: &otd_engine::panel::Widget) -> egui::Rect {
    egui::Rect::from_min_size(
        area.min + egui::vec2(w.rect[0] * area.width(), w.rect[1] * area.height()),
        egui::vec2(w.rect[2] * area.width(), w.rect[3] * area.height()),
    )
}

pub(crate) fn draw_panel_comp(
    ui: &mut egui::Ui,
    app: &mut OtdApp,
    comp: NodeId,
    area: egui::Rect,
    widgets: &[otd_engine::panel::Widget],
    interact: bool,
) -> Vec<(NodeId, f64)> {
    draw_panel_comp_look(&app.graph, comp, ui.painter(), area);
    draw_panel(ui, widgets, area, Some(app), interact)
}

fn draw_panel_comp_look(
    graph: &Graph,
    comp: NodeId,
    painter: &egui::Painter,
    area: egui::Rect,
) {
    let node = graph.node(comp);
    let ctx = otd_core::EvalContext::default();
    let f = |key: &str, default: f64| {
        node.param(key)
            .map(|p| p.eval(&ctx).as_f64())
            .unwrap_or(default)
    };
    let opacity = f("opacity", 1.0) as f32;
    let alpha = (opacity * f("bgalpha", 1.0) as f32).clamp(0.0, 1.0);
    if alpha <= 0.001 {
        return;
    }
    let color = egui::Color32::from_rgba_unmultiplied(
        (f("bgcolorr", 0.08) * 255.0) as u8,
        (f("bgcolorg", 0.09) * 255.0) as u8,
        (f("bgcolorb", 0.11) * 255.0) as u8,
        (alpha * 255.0) as u8,
    );
    painter.rect_filled(area, 0.0, color);
}

fn draw_panel_widget_look(painter: &egui::Painter, rect: egui::Rect, w: &otd_engine::panel::Widget) {
    let alpha = (w.opacity * w.bg[3]).clamp(0.0, 1.0);
    if alpha <= 0.001 {
        return;
    }
    let fill = egui::Color32::from_rgba_unmultiplied(
        (w.bg[0] * 255.0) as u8,
        (w.bg[1] * 255.0) as u8,
        (w.bg[2] * 255.0) as u8,
        (alpha * 255.0) as u8,
    );
    painter.rect_filled(rect, 4.0, fill);
}

pub(crate) fn draw_panel_texture(
    painter: &egui::Painter,
    rect: egui::Rect,
    tid: TextureId,
    size: [u32; 2],
) {
    let aspect = size[0] as f32 / size[1].max(1) as f32;
    let draw = if aspect > rect.width() / rect.height() {
        let h = rect.width() / aspect;
        egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), h))
    } else {
        let w = rect.height() * aspect;
        egui::Rect::from_center_size(rect.center(), egui::vec2(w, rect.height()))
    };
    painter.image(
        tid,
        draw,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

pub(crate) fn draw_panel(
    ui: &mut egui::Ui,
    widgets: &[otd_engine::panel::Widget],
    area: egui::Rect,
    mut app: Option<&mut OtdApp>,
    interact: bool,
) -> Vec<(NodeId, f64)> {
    use otd_engine::panel::Kind;
    let mut sorted: Vec<&otd_engine::panel::Widget> = widgets.iter().collect();
    sorted.sort_by(|a, b| {
        a.layer
            .partial_cmp(&b.layer)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut hits: Vec<(otd_engine::panel::Widget, egui::Rect)> = Vec::new();
    let mut changed = Vec::new();
    for w in sorted {
        if !w.display {
            continue;
        }
        let rect = panel_widget_rect(area, w);
        if !ui.max_rect().intersects(rect) {
            continue;
        }
        if matches!(w.kind, Kind::Container) {
            if let Some(app) = app.as_mut() {
                let nested =
                    otd_engine::panel::panel_display_widgets(&app.graph, w.id);
                changed.extend(draw_panel_comp(
                    ui, app, w.id, rect, &nested, interact,
                ));
            } else {
                draw_panel_widget_look(ui.painter(), rect, w);
            }
            continue;
        }
        if let Some(app) = app.as_mut() {
            if let Some(top) = crate::canvas::comp_display::comp_view_top(&app.graph, w.id) {
                if let Some((tid, size)) = app.thumbnail(top) {
                    draw_panel_texture(ui.painter(), rect, tid, size);
                }
            }
        }
        if interact && w.enabled {
            hits.push(((*w).clone(), rect));
        }
    }
    if !interact {
        return changed;
    }
    for (w, rect) in hits.into_iter().rev() {
        let child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        match w.kind {
            Kind::Button => {
                let on = w.value >= 0.5;
                let r = child.interact(
                    rect,
                    child.id().with(("panel-btn", w.id)),
                    egui::Sense::click(),
                );
                if w.momentary {
                    let down = r.is_pointer_button_down_on();
                    if down != on {
                        changed.push((w.id, if down { 1.0 } else { 0.0 }));
                    }
                } else if r.clicked() {
                    changed.push((w.id, if on { 0.0 } else { 1.0 }));
                }
            }
            Kind::Slider => {
                let (lo, hi) = w.range;
                let r = child.interact(
                    rect,
                    child.id().with(("panel-slider", w.id)),
                    egui::Sense::click_and_drag(),
                );
                if r.clicked() || r.dragged() {
                    let t = if w.vertical {
                        1.0 - ((r.interact_pointer_pos().unwrap_or(rect.center()).y - rect.min.y)
                            / rect.height())
                    } else {
                        (r.interact_pointer_pos().unwrap_or(rect.center()).x - rect.min.x)
                            / rect.width()
                    };
                    let value = lo + (hi - lo) * t.clamp(0.0, 1.0) as f64;
                    changed.push((w.id, value));
                }
            }
            _ => {}
        }
    }
    changed
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
        let roots = cook_roots(&graph, Some(out), &[spare], root);
        assert!(roots.contains(&spare));

        // Panned away: nothing wants it, so nothing cooks it.
        let roots = cook_roots(&graph, Some(out), &[], root);
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
        let performing = cook_roots(&graph, Some(out), &[], graph.root());
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
        assert!(cook_roots(&graph, Some(out), &[], graph.root()).is_empty());
    }
}
