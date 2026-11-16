use std::path::PathBuf;

use otd_core::NodeId;

use super::{CreateDialog, OtdApp};

#[derive(Clone, Debug)]
pub(crate) enum GraphCommand {
    DeleteSelection,
    PasteAt([f32; 2]),
    Disconnect {
        dst: NodeId,
        input: usize,
    },
    Connect {
        from: NodeId,
        output: usize,
        dst: NodeId,
        input: usize,
    },
    Enter(NodeId),
    Leave,
    CreateOperator {
        type_name: String,
        dialog: CreateDialog,
    },
    AddEffect {
        type_name: String,
    },
    RemoveCustomParam {
        node: NodeId,
        key: String,
    },
    AssignParamOpRef {
        target: NodeId,
        key: String,
        source: NodeId,
    },
}

impl OtdApp {
    pub(crate) fn queue_delete_selection(&mut self) {
        self.push_graph_command(GraphCommand::DeleteSelection);
    }

    pub(crate) fn queue_paste_clipboard(&mut self, origin: [f32; 2]) {
        if self.document.node_clipboard.nodes.is_empty() {
            return;
        }
        self.push_graph_command(GraphCommand::PasteAt(origin));
    }

    pub(crate) fn queue_disconnect(&mut self, dst: NodeId, input: usize) {
        self.push_graph_command(GraphCommand::Disconnect { dst, input });
    }

    pub(crate) fn queue_connect(&mut self, from: NodeId, output: usize, dst: NodeId, input: usize) {
        self.push_graph_command(GraphCommand::Connect {
            from,
            output,
            dst,
            input,
        });
    }

    pub(crate) fn queue_enter_network(&mut self, id: NodeId) {
        self.push_graph_command(GraphCommand::Enter(id));
    }

    pub(crate) fn queue_leave_network(&mut self) {
        self.push_graph_command(GraphCommand::Leave);
    }

    pub(crate) fn queue_create_operator(&mut self, type_name: String, dialog: CreateDialog) {
        self.push_graph_command(GraphCommand::CreateOperator { type_name, dialog });
    }

    pub(crate) fn queue_add_effect(&mut self, type_name: &str) {
        self.push_graph_command(GraphCommand::AddEffect {
            type_name: type_name.to_string(),
        });
    }

    pub(crate) fn queue_remove_custom_param(&mut self, node: NodeId, key: String) {
        self.push_graph_command(GraphCommand::RemoveCustomParam { node, key });
    }

    pub(crate) fn queue_assign_param_op_ref(
        &mut self,
        target: NodeId,
        key: String,
        source: NodeId,
    ) {
        self.push_graph_command(GraphCommand::AssignParamOpRef {
            target,
            key,
            source,
        });
    }

    pub(crate) fn queue_media_drop(&mut self, paths: Vec<PathBuf>, at: Option<egui::Pos2>) {
        if paths.is_empty() {
            return;
        }
        self.pending_media_drop = Some((paths, at));
    }

    fn push_graph_command(&mut self, cmd: GraphCommand) {
        if matches!(cmd, GraphCommand::DeleteSelection)
            && self
                .graph_commands
                .iter()
                .any(|c| matches!(c, GraphCommand::DeleteSelection))
        {
            return;
        }
        self.graph_commands.push(cmd);
    }

    pub(crate) fn drain_graph_commands(&mut self) {
        if let Some((paths, at)) = self.pending_media_drop.take() {
            crate::media::accept(self, &paths, at);
        }
        let cmds = std::mem::take(&mut self.graph_commands);
        let had_edits = !cmds.is_empty();
        for cmd in cmds {
            match cmd {
                GraphCommand::DeleteSelection => self.delete_selected(),
                GraphCommand::PasteAt(origin) => self.apply_paste_clipboard(origin),
                GraphCommand::Disconnect { dst, input } => self.apply_disconnect(dst, input),
                GraphCommand::Connect {
                    from,
                    output,
                    dst,
                    input,
                } => self.apply_connect(from, output, dst, input),
                GraphCommand::Enter(id) => self.apply_enter_network(id),
                GraphCommand::Leave => self.apply_leave_network(),
                GraphCommand::CreateOperator { type_name, dialog } => {
                    let world_pos = dialog.world_pos;
                    if let Some(new_id) = self.create_node(&type_name, world_pos) {
                        self.connect_created_node(new_id, &dialog);
                    }
                }
                GraphCommand::AddEffect { type_name } => {
                    crate::media::apply_add_effect(self, &type_name);
                }
                GraphCommand::RemoveCustomParam { node, key } => {
                    self.edit("remove parameter");
                    self.document.history.end_gesture();
                    self.graph.remove_custom_param(node, &key);
                }
                GraphCommand::AssignParamOpRef {
                    target,
                    key,
                    source,
                } => crate::canvas::apply_param_reference(self, target, &key, source),
            }
        }
        if had_edits && self.transport.playing {
            self.background_engine.publish(self);
        }
    }
}
