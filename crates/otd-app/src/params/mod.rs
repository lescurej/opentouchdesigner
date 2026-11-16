//! The parameter panel.
//!
//! Parameter panel: Constant and Expression modes (TouchDesigner-style).
//! Legacy Export/Bind values in `.otd` files migrate to `ch()` expressions on open.

mod channels;
mod component;
mod constant_chop;
mod curve;
mod dat;
mod file;
mod script_shader;
mod show;

use otd_core::{ParamMode, Value};

use crate::app::OtdApp;

pub use show::show;

pub(crate) use component::{component_links, custom_param_editor};
pub(crate) use dat::{
    dat_node_viewer_editor, dat_uses_text_param_editor,
};

#[derive(Clone, Debug)]
pub struct ChannelDrag {
    pub op_path: String,
    pub channel: String,
    pub color_index: usize,
    pub preview_value: f32,
    pub preview_peak: f32,
}

pub fn channel_expr_snippet(op_path: &str, channel: &str) -> String {
    format!("op('{op_path}')['{channel}']")
}

pub fn copy_channel_expression(app: &mut OtdApp, ctx: &egui::Context, snippet: String) {
    app.document.channel_expr_clipboard = snippet.clone();
    ctx.copy_text(snippet.clone());
    app.document.status = format!("copied {snippet}");
}

pub(crate) fn migrate_legacy_modes(app: &mut OtdApp, id: otd_core::NodeId) {
    let keys: Vec<String> = app.graph.node(id).params.keys().cloned().collect();
    for key in keys {
        let p = app.graph.node(id).params.get(&key).unwrap();
        let mode = p.mode;
        if mode == ParamMode::Export {
            if let Some((op, ch)) = p.source_parts() {
                let expr = channel_expr_snippet(op, ch);
                app.graph.node_mut(id).params.get_mut(&key).unwrap().set_expression(&expr);
            }
        } else if mode == ParamMode::Bind {
            if let Some((op, par)) = p.source_parts() {
                let expr = format!("par('{op}', '{par}')");
                app.graph.node_mut(id).params.get_mut(&key).unwrap().set_expression(&expr);
            }
        }
    }
}

pub(crate) fn format_value(v: &Value) -> String {
    match v {
        Value::Float(f) => format!("{f:.4}"),
        other => other.as_str(),
    }
}
