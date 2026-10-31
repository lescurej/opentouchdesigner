use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use otd_core::{Connector, EvalContext, Family, Graph, NodeId, Value};
use otd_gpu::ops::WINDOW;

use crate::app::{draw_texture_stretch, OtdApp};
use crate::canvas::comp_display;
use crate::window_layout;

pub fn is_window_comp(graph: &Graph, id: NodeId) -> bool {
    graph.contains(id) && graph.node(id).op_type == WINDOW
}

fn param_eval(graph: &Graph, id: NodeId, key: &str) -> Option<otd_core::Value> {
    graph.node(id).param(key).map(|p| p.eval(&EvalContext::default()))
}

pub fn is_open(graph: &Graph, id: NodeId) -> bool {
    param_eval(graph, id, "open")
        .map(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn wants_performance(graph: &Graph, id: NodeId) -> bool {
    param_eval(graph, id, "performance")
        .map(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn display_top(graph: &Graph, window: NodeId) -> Option<NodeId> {
    if let Some(path) = param_eval(graph, window, "winop").map(|v| v.as_str().trim().to_string()) {
        if !path.is_empty() {
            if let Some(resolved) = graph.find_from(window, &path) {
                return top_from_operator(graph, resolved);
            }
        }
    }
    graph
        .connector_ops(window, Connector::Out)
        .first()
        .copied()
}

fn top_from_operator(graph: &Graph, id: NodeId) -> Option<NodeId> {
    match graph.node(id).family {
        Family::Top => Some(id),
        Family::Comp => comp_display::comp_view_top(graph, id).or_else(|| {
            graph.connector_ops(id, Connector::Out).first().copied()
        }),
        _ => None,
    }
}

pub fn resolve_perform_ui(graph: &Graph, main_perform_ui_path: &str) -> Option<NodeId> {
    let path = main_perform_ui_path.trim();
    if path.is_empty() {
        return None;
    }
    graph.find(path).filter(|id| {
        graph
            .get(*id)
            .is_some_and(|n| n.family == Family::Comp)
    })
}

pub fn list_window_comps(graph: &Graph) -> Vec<NodeId> {
    graph
        .walk()
        .into_iter()
        .filter(|id| is_window_comp(graph, *id))
        .collect()
}

pub fn perform_display_top(
    graph: &Graph,
    main_perform_window: Option<NodeId>,
    perform_ui: Option<NodeId>,
) -> Option<NodeId> {
    if let Some(w) = main_perform_window.filter(|id| is_window_comp(graph, *id)) {
        if let Some(top) = display_top(graph, w) {
            return Some(top);
        }
    }
    let comp = perform_ui.filter(|v| graph.contains(*v) && graph.node(*v).family == Family::Comp)?;
    comp_display::comp_view_top(graph, comp).or_else(|| {
        graph
            .connector_ops(comp, Connector::Out)
            .first()
            .copied()
    })
}

pub fn list_perform_ui_containers(graph: &Graph) -> Vec<NodeId> {
    let mut ids: Vec<NodeId> = graph
        .walk()
        .into_iter()
        .filter(|id| graph.get(*id).is_some_and(|n| n.op_type == otd_engine::panel::CONTAINER))
        .collect();
    ids.sort_by_key(|id| graph.path(*id));
    ids
}

pub fn append_cook_roots(graph: &Graph, roots: &mut Vec<NodeId>) {
    for id in graph.walk() {
        if !is_window_comp(graph, id) || !is_open(graph, id) {
            continue;
        }
        if let Some(top) = display_top(graph, id) {
            if !roots.contains(&top) {
                roots.push(top);
            }
        }
    }
}

impl OtdApp {
    pub fn set_window_open(&mut self, id: NodeId, open: bool) {
        if !is_window_comp(&self.graph, id) {
            return;
        }
        self.edit("window open");
        self.history.end_gesture();
        let _ = self.graph.set_param(id, "open", Value::Bool(open));
    }

    pub fn set_main_perform_window(&mut self, id: NodeId) {
        if !is_window_comp(&self.graph, id) {
            return;
        }
        self.main_perform_window = Some(id);
        self.main_perform_window_path = self.graph.path(id);
    }

    pub fn sync_main_perform_from_path(&mut self) {
        if self.main_perform_window.is_some() {
            return;
        }
        if self.main_perform_window_path.is_empty() {
            return;
        }
        self.main_perform_window = self.graph.find(&self.main_perform_window_path);
    }

    pub fn sync_main_perform_ui_from_path(&mut self) {
        if self.main_perform_ui_path.is_empty() {
            return;
        }
        if let Some(id) = self.graph.find(&self.main_perform_ui_path) {
            self.viewer = Some(id);
        }
    }

    pub fn apply_project_settings(&mut self, settings: &otd_core::ProjectSettings) {
        self.main_perform_window_path = settings.main_perform_window.clone();
        self.main_perform_ui_path = settings.main_perform_ui.clone();
        self.main_perform_window = None;
        self.sync_main_perform_from_path();
        self.sync_main_perform_ui_from_path();
        self.start_in_perform_mode = settings.start_in_perform_mode;
        self.designer_custom_position = settings.designer_custom_position;
        self.designer_x = settings.designer_x;
        self.designer_y = settings.designer_y;
        self.designer_w = settings.designer_w;
        self.designer_h = settings.designer_h;
        self.designer_position_pending = settings.designer_custom_position;
        if settings.start_in_perform_mode {
            self.perform = true;
        }
    }

    pub fn project_settings(&self) -> otd_core::ProjectSettings {
        let main_perform_window = self
            .main_perform_window
            .map(|id| self.graph.path(id))
            .unwrap_or_else(|| self.main_perform_window_path.clone());
        let main_perform_ui = self
            .perform_ui()
            .map(|id| self.graph.path(id))
            .unwrap_or_else(|| self.main_perform_ui_path.clone());
        otd_core::ProjectSettings {
            main_perform_window,
            main_perform_ui,
            start_in_perform_mode: self.start_in_perform_mode,
            designer_custom_position: self.designer_custom_position,
            designer_x: self.designer_x,
            designer_y: self.designer_y,
            designer_w: self.designer_w,
            designer_h: self.designer_h,
        }
    }

    pub fn apply_designer_window_position(&self, ctx: &egui::Context) {
        if !self.designer_custom_position {
            return;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            self.designer_x,
            self.designer_y,
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            self.designer_w,
            self.designer_h,
        )));
    }

    pub fn capture_designer_window_position(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            let vp = i.viewport();
            if let Some(pos) = vp.outer_rect.map(|r| r.min) {
                self.designer_x = pos.x;
                self.designer_y = pos.y;
            }
            if let Some(size) = vp.inner_rect.map(|r| r.size()) {
                self.designer_w = size.x;
                self.designer_h = size.y;
            }
        });
        self.designer_custom_position = true;
    }

    pub fn sync_window_comp_performance(&mut self) {
        for id in self.graph.walk() {
            if !is_window_comp(&self.graph, id) || !wants_performance(&self.graph, id) {
                continue;
            }
            if self.main_perform_window != Some(id) {
                self.set_main_perform_window(id);
            }
        }
    }

    pub fn window_comp_viewports(&mut self, ctx: &egui::Context) {
        self.sync_window_comp_performance();

        let root_monitor = ctx.input(|i| i.viewport().monitor_size);
        let pointer = ctx.input(|i| i.pointer.interact_pos());

        let windows: Vec<NodeId> = self
            .graph
            .walk()
            .into_iter()
            .filter(|id| is_window_comp(&self.graph, *id) && is_open(&self.graph, *id))
            .collect();

        for id in windows {
            let closed = self
                .window_close_flags
                .get(&id)
                .is_some_and(|f| f.swap(false, Ordering::Relaxed));
            if closed {
                self.edit("window close");
                self.history.end_gesture();
                let _ = self.graph.set_param(id, "open", Value::Bool(false));
                self.window_was_open.insert(id, false);
                continue;
            }

            let open_now = is_open(&self.graph, id);
            let was_open = self.window_was_open.get(&id).copied().unwrap_or(false);
            if open_now && !was_open {
                if let Some(p) = pointer {
                    self.window_open_anchors.insert(id, p);
                }
            }
            self.window_was_open.insert(id, open_now);

            let cfg = window_layout::window_comp_config(&self.graph, id);
            let anchor = self.window_open_anchors.get(&id).copied().or(pointer);
            let tex_top = display_top(&self.graph, id);
            let tex = tex_top.and_then(|top| self.thumbnail(top));
            let tex_size = tex.as_ref().map(|(_, s)| *s);
            let builder = window_layout::build_viewport_builder(
                &cfg,
                tex_size,
                anchor,
                root_monitor,
            );
            let closed_flag = self.window_comp_closed(id);
            let stretch = cfg.stretch;
            let hide_cursor = !cfg.cursor_visible;
            let close_escape = cfg.close_on_escape;

            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of(("otd-window-comp", id)),
                builder,
                move |ui, _class| {
                    if hide_cursor {
                        ui.ctx().output_mut(|o| o.cursor_icon = egui::CursorIcon::None);
                    }
                    if close_escape && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                        closed_flag.store(true, Ordering::Relaxed);
                    }
                    egui::CentralPanel::no_frame()
                        .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
                        .show(ui, |ui| {
                            if let Some((tid, size)) = tex {
                                draw_texture_stretch(ui, tid, size, stretch);
                            }
                        });
                    if ui.ctx().input(|i| i.viewport().close_requested()) {
                        closed_flag.store(true, Ordering::Relaxed);
                    }
                },
            );
        }
    }

    fn window_comp_closed(&mut self, id: NodeId) -> Arc<AtomicBool> {
        self.window_close_flags
            .entry(id)
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }
}
