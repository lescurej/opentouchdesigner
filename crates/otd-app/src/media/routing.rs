use std::path::{Path, PathBuf};

use otd_core::{NodeId, Value};

use crate::app::OtdApp;

use super::classify::{classify, DropKind};
use super::layout::{default_pos, free_spot, sanitise};

pub fn apply_add_effect(app: &mut OtdApp, type_name: &str) {
    let Some(src) = app.canvas.selected.filter(|s| app.graph.contains(*s)) else {
        app.document.status = "select an operator first — the effect goes after it".into();
        return;
    };
    let p = app.graph.node(src).pos;
    let pos = free_spot(app, egui::vec2(p[0] + crate::canvas::NODE_W + 40.0, p[1]));
    let Some(id) = app.create_node(type_name, pos) else {
        return;
    };
    let by_param = match type_name {
        t if t == otd_gpu::ops::FEEDBACK => Some("target"),
        t if t == otd_gpu::ops::SELECT => Some("top"),
        _ => None,
    };
    if let Some(key) = by_param {
        let source = app.graph.node(src).name.clone();
        let _ = app.graph.set_param(id, key, Value::Str(source.clone()));
        app.graph.node_mut(id).flags.viewer = true;
        app.document.status = format!("{} {key} = {source}", app.graph.node(id).name);
        return;
    }

    match app.graph.connect(src, id, 0) {
        Ok(()) => {
            app.graph.node_mut(id).flags.viewer = true;
            app.document.status = format!(
                "{} -> {}",
                app.graph.node(src).name,
                app.graph.node(id).name
            );
        }
        Err(e) => app.document.status = format!("{} does not take that input: {e}", app.graph.node(id).name),
    }
}

pub fn accept(app: &mut OtdApp, paths: &[PathBuf], at: Option<egui::Pos2>) {
    let touchdesigner: Vec<PathBuf> = paths
        .iter()
        .filter(|p| classify(p) == DropKind::TouchDesigner)
        .cloned()
        .collect();
    if !touchdesigner.is_empty() {
        for path in &touchdesigner {
            app.open_td_import(path.clone());
        }
        let rest: Vec<PathBuf> = paths
            .iter()
            .filter(|p| classify(p) != DropKind::TouchDesigner)
            .cloned()
            .collect();
        if rest.is_empty() {
            return;
        }
        accept(app, &rest, at);
        return;
    }

    if let Some(project) = paths.iter().find(|p| classify(p) == DropKind::Project) {
        if paths.len() > 1 {
            app.document.status = format!(
                "opened {} and ignored the rest — a project replaces the network",
                project.display()
            );
        }
        let project = project.clone();
        app.open(Some(project));
        return;
    }

    if let (Some(pointer), 1) = (at, paths.len()) {
        if let Some(target) = crate::canvas::node_at(app, pointer) {
            if retarget(app, target, &paths[0]) {
                return;
            }
        }
    }

    let base = match at {
        Some(p) if app.canvas.canvas_rect.contains(p) => app.canvas.view.to_world(app.canvas.canvas_rect.min, p),
        _ => default_pos(app),
    };

    let mut added = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let pos = free_spot(
            app,
            base + egui::vec2(0.0, i as f32 * (crate::canvas::NODE_H + 28.0)),
        );
        match add_one(app, path, pos) {
            Some(id) => {
                added += 1;
                app.canvas.selected = Some(id);
            }
            None => skipped.push(
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        }
    }

    if !skipped.is_empty() {
        app.document.status = format!(
            "ignored {} — movies, images, audio, shaders, tables, fonts, .otd, .otdc, .tox and \
             .toe are what this reads",
            skipped.join(", ")
        );
    } else if added > 1 {
        app.document.status = format!("added {added} operators");
    }
}

pub(crate) fn tune_dropped_movie(app: &mut OtdApp, id: NodeId) {
    let _ = app
        .graph
        .set_param(id, "playmode", Value::Str("sequential".into()));
    let _ = app.graph.set_param(id, "play", Value::Bool(true));
    let _ = app.graph.set_param(id, "prereadframes", Value::Int(24));
    let _ = app.graph.set_param(id, "hwdecode", Value::Bool(true));
}

pub(crate) fn add_one(app: &mut OtdApp, path: &Path, pos: egui::Vec2) -> Option<NodeId> {
    let kind = classify(path);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "media".into());
    let stored = stored_path(app, path);

    match kind {
        DropKind::Image | DropKind::Movie => {
            let id = create_named(app, otd_gpu::ops::MOVIE_IN, pos, &stem)?;
            let _ = app.graph.set_param(id, "file", Value::Str(stored));
            if kind == DropKind::Movie {
                tune_dropped_movie(app, id);
            }
            app.graph.node_mut(id).flags.viewer = true;
            app.queue_texture_sync(id);
            app.document.status = format!(
                "{} — drop another on this node to swap the clip",
                app.graph.node(id).name
            );
            Some(id)
        }
        DropKind::Audio => {
            let id = create_named(app, otd_chop::io::AUDIO_FILE, pos, &stem)?;
            let _ = app.graph.set_param(id, "file", Value::Str(stored));
            app.document.status = if path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("wav"))
                .unwrap_or(false)
            {
                format!(
                    "{} — drag a channel from the parameter panel onto any parameter",
                    app.graph.node(id).name
                )
            } else {
                format!(
                    "{}: Audio File In reads WAV only — convert it, or use the movie's own audio",
                    app.graph.node(id).name
                )
            };
            Some(id)
        }
        DropKind::Component => {
            let id = app.import_component_at(path)?;
            app.graph.node_mut_quiet(id).pos = app.snap_world_pos([pos.x, pos.y]);
            app.set_perform_ui(id);
            Some(id)
        }
        DropKind::Isf => {
            let source = read_text(path)?;
            let isf = otd_gpu::isf::import(&source)
                .map_err(|e| app.document.status = format!("ISF import failed: {e}"))
                .ok()?;
            let id = create_named(app, otd_gpu::ops::GLSL, pos, &stem)?;
            let count = isf.params.len();
            otd_gpu::isf::apply(&mut app.graph, id, &isf);
            app.graph.node_mut(id).flags.viewer = true;
            app.document.status = format!("{stem} — {count} parameter(s)");
            Some(id)
        }
        DropKind::Shader(language) => {
            let source = read_text(path)?;
            let id = create_named(app, otd_gpu::ops::GLSL, pos, &stem)?;
            let _ = app
                .graph
                .set_param(id, "language", Value::Str(language.into()));
            let _ = app.graph.set_param(id, "source", Value::Str(source));
            app.graph.node_mut(id).flags.viewer = true;
            Some(id)
        }
        DropKind::Table(delimiter) => {
            let text = read_text(path)?;
            let id = create_named(app, otd_dat::ops::TABLE, pos, &stem)?;
            let _ = app.graph.set_param(id, "text", Value::Str(text));
            let _ = app
                .graph
                .set_param(id, "delimiter", Value::Str(delimiter.into()));
            Some(id)
        }
        DropKind::Text => {
            let text = read_text(path)?;
            let id = create_named(app, otd_dat::ops::TEXT, pos, &stem)?;
            let _ = app.graph.set_param(id, "text", Value::Str(text));
            Some(id)
        }
        DropKind::Font => {
            let id = create_named(app, otd_gpu::ops::TEXT, pos, &stem)?;
            let _ = app.graph.set_param(id, "font", Value::Str(stored));
            let _ = app.graph.set_param(id, "text", Value::Str(stem));
            app.graph.node_mut(id).flags.viewer = true;
            Some(id)
        }
        DropKind::Project | DropKind::TouchDesigner | DropKind::Unknown => None,
    }
}

fn retarget(app: &mut OtdApp, target: NodeId, path: &Path) -> bool {
    let kind = classify(path);
    if !kind.is_media() {
        return false;
    }
    let op_type = app.graph.node(target).op_type.clone();
    let stored = stored_path(app, path);

    let takes_file = matches!(
        (op_type.as_str(), kind),
        (otd_gpu::ops::MOVIE_IN, DropKind::Image | DropKind::Movie)
            | (otd_chop::io::AUDIO_FILE, DropKind::Audio)
    );
    if takes_file {
        app.edit("set file");
        app.document.history.end_gesture();
        let _ = app.graph.set_param(target, "file", Value::Str(stored));
        if op_type == otd_gpu::ops::MOVIE_IN && kind == DropKind::Movie {
            tune_dropped_movie(app, target);
        }
        app.graph.node_mut(target).flags.viewer = true;
        app.canvas.selected = Some(target);
        app.document.status = format!(
            "{} -> {}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            app.graph.node(target).name
        );
        return true;
    }

    let wanted = if kind == DropKind::Audio {
        otd_core::Family::Chop
    } else {
        otd_core::Family::Top
    };
    let node = app.graph.node(target);
    let slot = node.inputs.iter().enumerate().position(|(i, s)| {
        s.is_none() && node.input_families.get(i).copied().unwrap_or(node.family) == wanted
    });
    let Some(slot) = slot else {
        return false;
    };

    let p = app.graph.node(target).pos;
    let pos = free_spot(
        app,
        egui::vec2(
            p[0] - crate::canvas::NODE_W - 40.0,
            p[1] + slot as f32 * 24.0,
        ),
    );
    let Some(id) = add_one(app, path, pos) else {
        return false;
    };
    match app.graph.connect(id, target, slot) {
        Ok(()) => {
            app.graph.node_mut(target).flags.viewer = true;
            app.document.status = format!(
                "{} -> {}",
                app.graph.node(id).name,
                app.graph.node(target).name
            );
        }
        Err(e) => app.document.status = e.to_string(),
    }
    true
}

pub fn stored_path(app: &OtdApp, path: &Path) -> String {
    if let Some(base) = app.graph.base_dir() {
        if let Ok(rel) = path.strip_prefix(base) {
            return rel.to_string_lossy().to_string();
        }
    }
    path.to_string_lossy().to_string()
}

const MAX_TEXT_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) fn read_text(path: &Path) -> Option<String> {
    match std::fs::metadata(path) {
        Ok(m) if m.len() > MAX_TEXT_BYTES => return None,
        Ok(_) => {}
        Err(_) => return None,
    }
    std::fs::read_to_string(path).ok()
}

pub(crate) fn create_named(app: &mut OtdApp, type_name: &str, pos: egui::Vec2, hint: &str) -> Option<NodeId> {
    let id = app.create_node(type_name, pos)?;
    if let Some(name) = sanitise(hint) {
        let parent = app.graph.node(id).parent.unwrap_or(app.graph.root());
        if !app.graph.name_taken(parent, &name) {
            app.graph.node_mut_quiet(id).name = name;
        }
    }
    Some(id)
}