use super::super::*;

impl OtdApp {
    pub fn commit_canvas_rename(&mut self) {
        let Some((id, name)) = self.canvas.canvas_rename.take() else {
            return;
        };
        self.try_set_node_name(id, &name);
    }

    pub fn create_node(&mut self, type_name: &str, world_pos: egui::Vec2) -> Option<NodeId> {
        let def = self.registry.get(type_name)?.clone();
        self.edit("create");
        self.document.history.end_gesture();
        let parent = self.canvas.current;
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

    pub(crate) fn create_operator_tab_key(&mut self, ctx: &egui::Context) {
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
            .unwrap_or_else(|| self.canvas.canvas_rect.center());
        let origin = self.canvas.canvas_rect.min;
        let world_pos = if self.canvas.canvas_rect.is_positive() {
            self.canvas.view.to_world(origin, pointer)
        } else {
            egui::Vec2::ZERO
        };
        self.open_create_operator_dialog(world_pos);
    }

    pub(crate) fn open_create_operator_dialog(&mut self, world_pos: egui::Vec2) {
        let family = self
            .canvas.selected
            .filter(|id| self.graph.contains(*id))
            .map(|id| self.graph.node(id).family)
            .unwrap_or(otd_core::Family::Top);
        self.create_dialog = Some(CreateDialog {
            filter: String::new(),
            family,
            world_pos,
            focus: true,
            connect_from: self.canvas.selected,
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
            self.document.status = format!("name `{name}` already used here");
            return;
        }
        self.edit("rename");
        self.graph.node_mut_quiet(id).name = name.to_string();
        self.document.history.end_gesture();
    }
}