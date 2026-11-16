use otd_core::NodeId;

impl super::super::OtdApp {
    fn output_extra_roots(&self, perform_ui: Option<NodeId>) -> Vec<NodeId> {
        let mut extra = Vec::new();
        crate::window_comp::append_cook_roots(&self.graph, &mut extra);
        if let Some(top) = crate::window_comp::perform_display_top(
            &self.graph,
            self.perform_shell.main_perform_window,
            perform_ui,
        ) && !extra.contains(&top)
        {
            extra.push(top);
        }
        if let Some(comp) = perform_ui {
            crate::canvas::comp_display::append_perform_panel_cook_roots(
                &self.graph,
                comp,
                &mut extra,
            );
        }
        extra
    }

    fn editor_extra_roots(&self) -> Vec<NodeId> {
        let mut extra = Vec::new();
        crate::canvas::comp_display::append_network_display_roots(
            &self.graph,
            self.canvas.current,
            &self.canvas.visible,
            &mut extra,
        );
        crate::container_view::append_cook_roots(
            &self.graph,
            &self.canvas.open_container_views,
            &mut extra,
        );
        extra
    }

    pub(crate) fn cook_roots(&self, include_editor: bool) -> Vec<NodeId> {
        let perform_ui = self.perform_ui();
        otd_engine::app_cook_roots(otd_engine::CookRootsRequest {
            graph: &self.graph,
            perform_ui,
            visible: &self.canvas.visible,
            current: self.canvas.current,
            output_extra_roots: &self.output_extra_roots(perform_ui),
            editor_extra_roots: &self.editor_extra_roots(),
            include_editor,
        })
    }
}
