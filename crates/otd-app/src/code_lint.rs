use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use egui::Id;
use otd_core::{ChannelSource, Graph, NodeId};
use otd_engine::Engines;
use otd_engine::{LintSeverity, PythonLint};

const DEBOUNCE_SECS: f64 = 0.35;

struct Entry {
    pending_source: String,
    pending_hash: u64,
    scheduled_at: f64,
    checked_hash: u64,
    issues: Vec<PythonLint>,
}

#[derive(Default)]
pub struct CodeLintCache {
    entries: HashMap<Id, Entry>,
}

impl CodeLintCache {
    pub fn touch(&mut self, id: Id, source: &str, now: f64) {
        let hash = hash_str(source);
        let e = self.entries.entry(id).or_insert(Entry {
            pending_source: String::new(),
            pending_hash: 0,
            scheduled_at: now,
            checked_hash: hash,
            issues: Vec::new(),
        });
        if e.pending_hash != hash {
            e.pending_source = source.to_string();
            e.pending_hash = hash;
            e.scheduled_at = now;
        }
    }

    pub fn poll(
        &mut self,
        id: Id,
        now: f64,
        engines: &Engines,
        graph: &Graph,
        me_path: &str,
        filename: &str,
    ) {
        let Some(e) = self.entries.get_mut(&id) else {
            return;
        };
        if e.pending_hash == e.checked_hash {
            return;
        }
        if now - e.scheduled_at < DEBOUNCE_SECS {
            return;
        }
        let source = e.pending_source.clone();
        let hash = e.pending_hash;
        let issues = engines.lint_python(graph, me_path, &source, filename);
        if let Some(e) = self.entries.get_mut(&id) {
            e.checked_hash = hash;
            e.issues = issues;
        }
    }

    pub fn issues(&self, id: Id) -> &[PythonLint] {
        self.entries
            .get(&id)
            .map(|e| e.issues.as_slice())
            .unwrap_or(&[])
    }
}

fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub fn me_path_for_node(graph: &Graph, id: NodeId) -> String {
    if let Some(owner) = graph.node(id).callback_owner {
        return graph.path(owner);
    }
    graph.path(id)
}

pub fn operator_paths(graph: &Graph) -> Vec<String> {
    graph
        .walk()
        .into_iter()
        .filter(|id| *id != graph.root())
        .map(|id| graph.path(id))
        .collect()
}

pub fn channel_names_at(
    engines: &Engines,
    graph: &Graph,
    me_path: &str,
    op_ref: &str,
) -> Vec<String> {
    let resolved = otd_engine::lint::resolve_op_ref(me_path, op_ref);
    let Some(id) = graph.find(&resolved) else {
        return Vec::new();
    };
    if let Some(data) = engines.chop_data(id) {
        return data.channels.iter().map(|c| c.name.clone()).collect();
    }
    let net = otd_chop::Network {
        graph,
        chops: &engines.channels,
        panel: None,
    };
    net.channel_names(&resolved)
}

pub fn show_issues(ui: &mut egui::Ui, issues: &[PythonLint]) {
    if issues.is_empty() {
        return;
    }
    ui.add_space(4.0);
    for issue in issues.iter().take(12) {
        let color = match issue.severity {
            LintSeverity::Error => egui::Color32::from_rgb(230, 110, 110),
            LintSeverity::Warning => egui::Color32::from_rgb(230, 190, 110),
        };
        let prefix = match issue.severity {
            LintSeverity::Error => "error",
            LintSeverity::Warning => "warn",
        };
        let line_text = format!(
            "Line {}:{} — {prefix}: {}",
            issue.line, issue.column, issue.message
        );
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&line_text)
                    .small()
                    .monospace()
                    .color(color),
            );
            if ui
                .small_button("Copy")
                .on_hover_text("Copy this message to the system clipboard")
                .clicked()
            {
                ui.ctx().copy_text(line_text);
            }
        });
    }
    if issues.len() > 12 {
        ui.label(
            egui::RichText::new(format!("… and {} more", issues.len() - 12))
                .small()
                .weak(),
        );
    }
}
