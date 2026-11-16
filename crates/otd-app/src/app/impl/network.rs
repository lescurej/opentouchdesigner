use super::super::*;

impl OtdApp {
    pub(crate) fn apply_connect(
        &mut self,
        from: NodeId,
        output: usize,
        dst: NodeId,
        input: usize,
    ) {
        self.edit("wire");
        self.document.history.end_gesture();
        match self.graph.connect_from_output(from, output, dst, input) {
            Ok(()) => self.document.status.clear(),
            Err(e) => self.document.status = e.to_string(),
        }
    }

    pub(crate) fn apply_disconnect(&mut self, dst: NodeId, input: usize) {
        if !self.graph.contains(dst) || self.graph.node(dst).flags.lock {
            return;
        }
        self.edit("unwire");
        self.document.history.end_gesture();
        let _ = self.graph.disconnect(dst, input);
        if self.canvas.selected_wire == Some((dst, input)) {
            self.canvas.selected_wire = None;
        }
        self.document.status.clear();
    }

    pub fn apply_enter_network(&mut self, id: NodeId) {
        if self.graph.get(id).map(|n| n.family) == Some(otd_core::Family::Comp) {
            self.switch_network(id);
            self.clear_selection();
        }
    }

    pub fn apply_leave_network(&mut self) {
        if let Some(parent) = self.graph.get(self.canvas.current).and_then(|n| n.parent) {
            let was = self.canvas.current;
            self.switch_network(parent);
            self.select_only(was);
        }
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
                Ok(()) => self.document.status.clear(),
                Err(e) => self.document.status = e.to_string(),
            }
        } else if let Some(src) = dialog.connect_from.filter(|s| self.graph.contains(*s)) {
            match self.graph.connect(src, new_id, 0) {
                Ok(()) => self.document.status.clear(),
                Err(e) => self.document.status = e.to_string(),
            }
        }
    }

    pub(crate) fn rehome_current_before_remove(&mut self, roots: &[NodeId]) {
        for &root in roots {
            if !self.graph.contains(root) {
                continue;
            }
            if self.canvas.current != root
                && !crate::canvas::comp_display::is_descendant_of(&self.graph, root, self.canvas.current)
            {
                continue;
            }
            let parent = self
                .graph
                .node(root)
                .parent
                .filter(|p| self.graph.contains(*p))
                .unwrap_or_else(|| self.graph.root());
            self.canvas.network_views.insert(self.canvas.current, self.canvas.view);
            self.canvas.current = parent;
            self.canvas.view = self
                .canvas.network_views
                .get(&parent)
                .copied()
                .unwrap_or_default();
            return;
        }
    }

    pub(crate) fn switch_network(&mut self, id: NodeId) {
        if id == self.canvas.current || !self.graph.contains(id) {
            return;
        }
        self.canvas.network_views.insert(self.canvas.current, self.canvas.view);
        self.canvas.current = id;
        self.canvas.view = self.canvas.network_views.get(&id).copied().unwrap_or_default();
    }
}