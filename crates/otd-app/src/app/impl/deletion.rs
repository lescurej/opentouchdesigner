use super::super::*;

impl OtdApp {
    pub fn clear_network(&mut self) -> usize {
        self.assistant.reset_context();
        let doomed: Vec<NodeId> = self.graph.node(self.canvas.current).children.clone();
        if doomed.is_empty() {
            return 0;
        }
        self.edit("clear");
        self.document.history.end_gesture();
        self.clear_selection();
        self.delete_all(&doomed);
        self.perform_shell.main_perform_ui_path.clear();
        self.canvas.viewer = None;
        doomed.len()
    }

    pub(crate) fn collect_subtree(&self, root: NodeId) -> Vec<NodeId> {
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

    pub(crate) fn delete_all(&mut self, doomed: &[NodeId]) {
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
                self.perform_shell.main_perform_ui_path.clear();
                self.canvas.viewer = None;
            }
        }
        for id in removed {
            self.forget_node_runtime(id);
        }
        for (src, consumer, slot) in splices {
            let _ = self.graph.connect(src, consumer, slot);
        }
        self.repair_app_graph_pointers();
        self.canvas.drag = None;
    }

    pub fn delete_selected(&mut self) {
        if let Some((dst, input)) = self.canvas.selected_wire {
            if self.graph.contains(dst) && !self.graph.node(dst).flags.lock {
                self.apply_disconnect(dst, input);
            }
            return;
        }
        if self.canvas.selection.is_empty() {
            return;
        }
        let doomed: Vec<NodeId> = self
            .canvas.selection
            .iter()
            .copied()
            .filter(|id| self.graph.contains(*id) && !self.graph.node(*id).flags.lock)
            .collect();
        if doomed.is_empty() {
            self.document.status = "Selection is locked".into();
            return;
        }
        self.edit("delete");
        self.document.history.end_gesture();
        for id in &doomed {
            self.canvas.selection.remove(id);
        }
        self.canvas.selected = self
            .canvas.selected
            .filter(|s| self.canvas.selection.contains(s))
            .or_else(|| self.canvas.selection.iter().next().copied());
        self.delete_all(&doomed);
    }

    pub(crate) fn forget_node_runtime(&mut self, id: NodeId) {
        self.engine.forget_node(id);
        self.free_thumbnail(id);
        self.canvas.viewer_top_pan.remove(&id);
        self.canvas.viewer_top_zoom.remove(&id);
        self.canvas.viewer_dat_scroll.remove(&id);
        self.param_inspector_page.remove(&id);
        self.canvas.callback_embedded.remove(&id);
        self.perform_shell.window_close_flags.remove(&id);
        self.perform_shell.window_open_anchors.remove(&id);
        self.perform_shell.window_was_open.remove(&id);
    }

    pub(crate) fn free_thumbnail(&mut self, id: NodeId) {
        if let Some(view) = self.thumbs.remove(&id) {
            self.render_state
                .renderer
                .write()
                .free_texture(&view.egui_id);
        }
        self.pending_texture_sync.retain(|pending| *pending != id);
    }
}