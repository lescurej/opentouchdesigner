use super::super::*;

impl OtdApp {
    pub fn apply_paste_clipboard(&mut self, origin: [f32; 2]) {
        if self.document.node_clipboard.nodes.is_empty() {
            return;
        }
        self.document.paste_count += 1;
        let nudge = (self.document.paste_count.saturating_sub(1) as f32) * 24.0;
        let at = [origin[0] + nudge, origin[1] + nudge];
        self.edit("paste");
        self.document.history.end_gesture();
        match paste(
            &mut self.graph,
            self.canvas.current,
            &self.registry,
            &self.document.node_clipboard,
            at,
        ) {
            Ok(ids) => {
                self.clear_selection();
                for id in &ids {
                    self.canvas.selection.insert(*id);
                }
                self.canvas.selected = ids.last().copied();
                self.document.status = format!("Pasted {} operator(s)", ids.len());
            }
            Err(e) => self.document.status = format!("Paste failed: {e}"),
        }
        self.repair_app_graph_pointers();
    }

    pub(crate) fn clipboard_keys(&mut self, ctx: &egui::Context) {
        crate::canvas::input::operator_clipboard_keys(self, ctx);
    }

    pub fn copy_selection(&mut self, ctx: &egui::Context) {
        let ids: Vec<NodeId> = if !self.canvas.selection.is_empty() {
            self.canvas.selection.iter().copied().collect()
        } else {
            self.canvas.selected.into_iter().collect()
        };
        if ids.is_empty() {
            return;
        }
        self.document.node_clipboard = copy_selection(&self.graph, self.canvas.current, &ids, &self.registry);
        if self.document.node_clipboard.nodes.is_empty() {
            self.document.status = "Nothing to copy (locked or outside this network)".into();
        } else {
            self.document.paste_count = 0;
            self.document.status = format!("Copied {} operator(s)", self.document.node_clipboard.nodes.len());
            if let Some(text) = otd_core::encode_operator_clipboard(&self.document.node_clipboard) {
                ctx.copy_text(text);
            }
        }
    }

    pub fn cut_selection(&mut self, ctx: &egui::Context) {
        self.copy_selection(ctx);
        if !self.document.node_clipboard.nodes.is_empty() {
            self.queue_delete_selection();
        }
    }

    pub fn has_operator_clipboard(&self) -> bool {
        !self.document.node_clipboard.nodes.is_empty()
    }

    pub fn load_operator_clipboard_from_text(&mut self, text: &str) -> bool {
        let Some(clip) = otd_core::decode_operator_clipboard(text) else {
            return false;
        };
        if clip.nodes.is_empty() {
            return false;
        }
        self.document.node_clipboard = clip;
        self.document.paste_count = 0;
        true
    }
}