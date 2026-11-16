//! Drop a `.toe` / `.tox` to import it. The parity window opens only when
//! operators or parameters are missing.

use std::path::PathBuf;

use egui::text::{CCursor, CCursorRange};
use egui::{self, RichText};

use crate::app::OtdApp;

const REPORT_EDIT_ID: &str = "td_import_report_body";

#[derive(Clone)]
pub struct TdImportDialog {
    pub path: PathBuf,
    pub body: String,
    pub failed: bool,
    pub decode: Option<otd_td::DecodedExpansion>,
}

impl OtdApp {
    pub fn open_td_import(&mut self, path: PathBuf) {
        match otd_td::decode_with_fallback(&path) {
            Ok(decode) => {
                let parity = otd_td::analyze_expansion(&decode.files, &self.registry);
                let source = match &decode.source {
                    otd_td::DecodeSource::Native(meta) => format!(
                        "native Rust decoder ({} segment(s), kinds {:?}, {} ms)",
                        meta.segments, meta.kinds, decode.decode_ms
                    ),
                    otd_td::DecodeSource::ToeExpand { tool, .. } => {
                        format!("toeexpand ({}, {} ms)", tool.display(), decode.decode_ms)
                    }
                };
                let notes = if !parity.has_gaps() {
                    Vec::new()
                } else if parity.missing.is_empty() {
                    vec!["Some parameters have no matching OpenTouchDesigner parameter.".into()]
                } else if parity.missing_params.is_empty() {
                    vec![
                        "Some operator types are missing — import keeps them as placeholders."
                            .into(),
                    ]
                } else {
                    vec!["Some operator types and parameters are missing.".into()]
                };
                let body = otd_td::format_report_text(
                    &path.display().to_string(),
                    &source,
                    &parity,
                    &notes,
                );
                let gaps = parity.has_gaps();
                self.td_import = Some(TdImportDialog {
                    path,
                    body,
                    failed: false,
                    decode: Some(decode),
                });
                if gaps {
                    self.document.status = "TouchDesigner import — missing operators or parameters".into();
                } else {
                    self.apply_td_graph_import();
                }
            }
            Err(fail) => {
                let mut body = String::from("Could not decode this TouchDesigner file.\n\n");
                for n in &fail.notes {
                    body.push_str(&format!("• {n}\n"));
                }
                body.push_str(
                    "\nFallback chain: native decode → toeexpand → this message.\n\
                     Install TouchDesigner for toeexpand, or inspect the file with WebToe.",
                );
                self.td_import = Some(TdImportDialog {
                    path,
                    body,
                    failed: true,
                    decode: None,
                });
                self.document.status = "TouchDesigner file — decode failed (see dialog)".into();
            }
        }
    }

    pub fn apply_td_graph_import(&mut self) -> bool {
        let Some(dialog) = self.td_import.take() else {
            return false;
        };
        let TdImportDialog {
            path,
            body,
            failed,
            decode,
        } = dialog;
        let Some(decode) = decode else {
            self.td_import = Some(TdImportDialog {
                path,
                body,
                failed,
                decode: None,
            });
            return false;
        };
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "imported".into());
        let base = match otd_td::expansion_filesystem_root(&path, &decode) {
            Ok(p) => p,
            Err(e) => {
                self.document.status = format!("Import failed: {e}");
                return false;
            }
        };
        self.edit("import TouchDesigner");
        self.document.history.end_gesture();
        let parent = self.canvas.current;
        match otd_td::import_into_graph(
            &mut self.graph,
            parent,
            &decode.files,
            &base,
            &self.registry,
            &name,
        ) {
            Ok(result) => {
                self.select_only(result.root);
                self.apply_enter_network(result.root);
                self.engine_rt().engines.sync_with_graph(&self.graph);
                let warn = if result.warnings.is_empty() {
                    String::new()
                } else {
                    format!(" ({} warnings)", result.warnings.len())
                };
                self.document.status = format!(
                    "Imported {name} — {} nodes, {} wires{warn}",
                    result.nodes, result.wires
                );
                for w in result.warnings.iter().take(5) {
                    self.graph.load_warnings.push(w.clone());
                }
                self.td_import_rect = None;
                true
            }
            Err(e) => {
                self.document.status = format!("Import failed: {e}");
                false
            }
        }
    }
}

/// Read-only report viewer: `TextEdit` so selection drag auto-scrolls; edits are reverted.
fn report_body(ui: &mut egui::Ui, body: &mut String) {
    let snapshot = body.clone();
    let id = egui::Id::new(REPORT_EDIT_ID);
    let response = ui.add(
        egui::TextEdit::multiline(body)
            .id(id)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .interactive(true),
    );
    if response.changed() {
        *body = snapshot;
    }
    if response.has_focus() && ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A)) {
        ui.ctx()
            .input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A));
        let end = body.chars().count();
        let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(end))));
        state.store(ui.ctx(), id);
    }
}

pub fn show(app: &mut OtdApp, ctx: &egui::Context) {
    if app.td_import.is_none() {
        app.td_import_rect = None;
        return;
    }
    let failed = app.td_import.as_ref().unwrap().failed;
    let title = if failed {
        "TouchDesigner import — could not decode"
    } else {
        "TouchDesigner import — OTD parity"
    };
    let path = app.td_import.as_ref().unwrap().path.display().to_string();
    let mut open = true;
    let mut import_clicked = false;
    let screen = ctx.content_rect();
    let max_size = egui::vec2(
        (screen.width() * 0.9).clamp(360.0, 760.0),
        (screen.height() * 0.86).max(240.0),
    );
    let scroll_h = (max_size.y - 132.0).clamp(100.0, 520.0);
    let window = egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_size([560.0, scroll_h + 120.0])
        .max_size(max_size)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(RichText::new(path).strong());
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("td_import_report")
                .max_height(scroll_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    if !failed {
                        ui.label(
                            "This file is missing operators or parameters. Import still loads the network \
                             into the current component: unknown types become placeholders, and unknown \
                             parameters are kept as custom parameters.",
                        );
                        ui.add_space(6.0);
                    }
                    if let Some(dialog) = app.td_import.as_mut() {
                        report_body(ui, &mut dialog.body);
                    }
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if !failed && ui.button("Import network").clicked() {
                    import_clicked = true;
                }
                if ui.button("Copy report").clicked() {
                    if let Some(dialog) = &app.td_import {
                        ui.ctx().copy_text(dialog.body.clone());
                    }
                }
                if ui.button("Close").clicked() {
                    app.td_import = None;
                }
            });
        });
    app.td_import_rect = window.as_ref().map(|r| r.response.rect);
    if import_clicked {
        app.apply_td_graph_import();
        open = false;
    }
    if !open {
        app.td_import = None;
        app.td_import_rect = None;
    }
}
