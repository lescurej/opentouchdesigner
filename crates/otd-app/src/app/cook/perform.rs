use egui::TextureId;
use otd_core::NodeId;

impl super::super::OtdApp {
    pub fn set_perform_ui(&mut self, id: NodeId) {
        if !self.graph.contains(id) {
            return;
        }
        if self.graph.node(id).family != otd_core::Family::Comp {
            self.document.status = "Perform UI must be a COMP".into();
            return;
        }
        self.edit("perform UI");
        self.document.history.end_gesture();
        self.perform_shell.main_perform_ui_path = self.graph.path(id);
        self.canvas.viewer = Some(id);
        self.queue_enter_network(id);
        self.select_only(id);
    }

    pub fn toggle_perform_ui(&mut self, id: NodeId) {
        if self.perform_ui() == Some(id) {
            self.edit("perform UI");
            self.document.history.end_gesture();
            self.perform_shell.main_perform_ui_path.clear();
            self.canvas.viewer = None;
        } else {
            self.set_perform_ui(id);
        }
    }

    pub fn perform_output_thumbnail(&mut self) -> Option<(TextureId, [u32; 2])> {
        let out = crate::window_comp::perform_display_top(
            &self.graph,
            self.perform_shell.main_perform_window,
            self.perform_ui(),
        )?;
        self.thumbnail(out)
    }
}
