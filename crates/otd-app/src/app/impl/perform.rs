use super::super::*;

impl OtdApp {
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
        self.perform_shell.perform = false;
        self.perform_shell.perform_edit = false;
        self.focus_perform_ui_for_edit();
    }

    pub fn perform_preview_only(&self) -> bool {
        self.perform_shell.perform && !self.perform_shell.perform_edit
    }

    pub fn perform_ui(&self) -> Option<NodeId> {
        crate::window_comp::resolve_perform_ui(&self.graph, &self.perform_shell.main_perform_ui_path)
    }

    pub fn recording(&self) -> bool {
        self.perform_shell.recorder
            .filter(|id| self.graph.contains(*id))
            .map(|id| self.graph.node(id).params["record"].value == Value::Bool(true))
            .unwrap_or(false)
    }

    pub fn toggle_record(&mut self) {
        if self.recording() {
            let id = self.perform_shell.recorder.unwrap();
            self.edit("record");
            let _ = self.graph.set_param(id, "record", Value::Bool(false));
            let file = self.graph.node(id).params["file"].value.as_str();
            self.document.status = format!("Recorded {file}");
            return;
        }
        let Some(comp) = self.perform_ui() else {
            self.document.status = "Nothing to record: designate a Perform UI container in Window Placement".into();
            return;
        };
        let Some(out_top) = self
            .graph
            .connector_ops(comp, otd_core::graph::Connector::Out)
            .first()
            .copied()
        else {
            self.document.status = "Perform COMP has no Out TOP to record".into();
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Movie", &["mp4", "mov"])
            .set_file_name("out.mp4")
            .save_file()
        else {
            return;
        };
        let id = match self.perform_shell.recorder.filter(|id| self.graph.contains(*id)) {
            Some(id) => id,
            None => {
                let pos = self.graph.node(comp).pos;
                let Some(id) =
                    self.create_node(otd_gpu::ops::MOVIE_OUT, egui::vec2(pos[0] + 180.0, pos[1]))
                else {
                    self.document.status = "Movie File Out is not registered".into();
                    return;
                };
                if let Err(e) = self.graph.connect(out_top, id, 0) {
                    self.document.status = format!("Could not wire the recorder: {e}");
                    return;
                }
                self.graph.node_mut(id).flags.render = true;
                self.perform_shell.recorder = Some(id);
                id
            }
        };
        self.edit("record");
        let _ = self
            .graph
            .set_param(id, "file", Value::Str(path.display().to_string()));
        let _ = self.graph.set_param(id, "record", Value::Bool(true));
        self.document.status = format!("Recording to {}", path.display());
    }

    pub fn view_panel_comp(&mut self, id: NodeId) {
        if !self.graph.contains(id) || self.graph.node(id).family != otd_core::Family::Comp {
            return;
        }
        if !otd_engine::panel::is_panel_type(&self.graph.node(id).op_type) {
            return;
        }
        if self.graph.node(id).op_type == otd_engine::panel::CONTAINER {
            self.canvas.open_container_views.insert(id);
            self.canvas.viewer = Some(id);
            return;
        }
        self.set_perform_ui(id);
        self.perform_shell.perform = true;
        self.perform_shell.perform_edit = false;
    }
}