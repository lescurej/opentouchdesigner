use super::super::*;

impl OtdApp {
    pub fn edit(&mut self, tag: &str) {
        self.document.document_dirty = true;
        self.document.history.checkpoint(&self.graph, tag);
    }

    pub(crate) fn repair_app_graph_pointers(&mut self) {
        self.canvas.selected = self.canvas.selected.filter(|id| self.graph.contains(*id));
        self.canvas.selection.retain(|id| self.graph.contains(*id));
        if self.canvas.selected.is_none() {
            self.canvas.selected = self.canvas.selection.iter().next().copied();
        }
        self.canvas.viewer = self.canvas.viewer.filter(|id| {
            self.graph.contains(*id) && self.graph.node(*id).family == otd_core::Family::Comp
        });
        if !self.perform_shell.main_perform_ui_path.is_empty()
            && self.graph.find(&self.perform_shell.main_perform_ui_path).is_none()
        {
            self.perform_shell.main_perform_ui_path.clear();
        }
        if !self.graph.contains(self.canvas.current) {
            self.canvas.current = self.graph.root();
            self.canvas.view = self
                .canvas.network_views
                .get(&self.canvas.current)
                .copied()
                .unwrap_or_default();
        }
        self.canvas.visible.retain(|id| self.graph.contains(*id));
        self.canvas.network_views.retain(|id, _| self.graph.contains(*id));
        self.canvas.open_container_views
            .retain(|id| self.graph.contains(*id));
    }

    pub(crate) fn restore(&mut self, graph: Graph) {
        self.graph = graph;
        self.repair_app_graph_pointers();
        self.canvas.drag = None;
    }

    pub fn redo(&mut self) {
        match self.document.history.redo(&self.graph) {
            Some(graph) => {
                self.restore(graph);
                self.document.status = "Redo".into();
            }
            None => self.document.status = "nothing to redo".into(),
        }
    }

    pub fn undo(&mut self) {
        match self.document.history.undo(&self.graph) {
            Some(graph) => {
                self.restore(graph);
                self.document.status = "Undo".into();
            }
            None => self.document.status = "nothing to undo".into(),
        }
    }
}