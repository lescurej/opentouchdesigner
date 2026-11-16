use std::path::PathBuf;

use otd_core::{NodeId, Value};

use crate::app::OtdApp;

use super::classify::{classify, DropKind, AUDIO_EXT, IMAGE_EXT, MOVIE_EXT};
use super::layout::default_pos;
use super::routing::{accept, create_named, stored_path};

pub fn handle_drops(app: &mut OtdApp, ctx: &egui::Context) {
    let dropped: Vec<PathBuf> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .filter_map(|f| f.path.clone())
            .collect()
    });
    if dropped.is_empty() {
        return;
    }
    let at = ctx.pointer_latest_pos();
    if crate::assistant::claim_drop(app, &dropped, at) {
        return;
    }
    app.queue_media_drop(dropped, at);
}

pub fn hover_overlay(app: &OtdApp, ctx: &egui::Context) {
    let hovered: Vec<PathBuf> = ctx.input(|i| {
        i.raw
            .hovered_files
            .iter()
            .filter_map(|f| f.path.clone())
            .collect()
    });
    if hovered.is_empty() {
        return;
    }
    let rect = if app.canvas.canvas_rect.is_positive() {
        app.canvas.canvas_rect
    } else {
        ctx.input(|i| i.raw.screen_rect.unwrap_or(egui::Rect::ZERO))
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("drop-overlay"),
    ));
    painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(170));
    painter.rect_stroke(
        rect.shrink(6.0),
        8.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 180, 240)),
        egui::StrokeKind::Inside,
    );

    let as_reference = crate::assistant::would_claim(app, &hovered, ctx.pointer_latest_pos());

    let mut y = rect.center().y - (hovered.len().min(8) as f32 * 24.0) / 2.0 - 22.0;
    painter.text(
        egui::pos2(rect.center().x, y),
        egui::Align2::CENTER_CENTER,
        match as_reference {
            true => "Drop to build this look",
            false => "Drop to add",
        },
        egui::FontId::proportional(20.0),
        egui::Color32::from_rgb(200, 220, 245),
    );
    y += 34.0;
    for path in hovered.iter().take(8) {
        let kind = classify(path);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let colour = if kind == DropKind::Unknown && !as_reference {
            egui::Color32::from_rgb(230, 130, 130)
        } else {
            egui::Color32::from_rgb(225, 228, 235)
        };
        let becomes = match as_reference {
            true => "reference image for the assistant",
            false => kind.describe(),
        };
        painter.text(
            egui::pos2(rect.center().x, y),
            egui::Align2::CENTER_CENTER,
            format!("{name}   ->   {becomes}"),
            egui::FontId::proportional(15.0),
            colour,
        );
        y += 24.0;
    }
    if hovered.len() > 8 {
        painter.text(
            egui::pos2(rect.center().x, y),
            egui::Align2::CENTER_CENTER,
            format!("…and {} more", hovered.len() - 8),
            egui::FontId::proportional(13.0),
            egui::Color32::from_gray(150),
        );
    }
}

pub fn import_dialog(app: &mut OtdApp) {
    let mut all: Vec<&str> = Vec::new();
    all.extend_from_slice(IMAGE_EXT);
    all.extend_from_slice(MOVIE_EXT);
    all.extend_from_slice(AUDIO_EXT);
    let picked = rfd::FileDialog::new()
        .set_title("Import media")
        .add_filter("Movies, images and audio", &all)
        .add_filter("Movies", MOVIE_EXT)
        .add_filter("Images", IMAGE_EXT)
        .add_filter("Audio", AUDIO_EXT)
        .add_filter("Anything", &["*"])
        .pick_files();
    if let Some(paths) = picked {
        accept(app, &paths, None);
    }
}

pub fn add_webcam(app: &mut OtdApp) {
    let pos = default_pos(app);
    if let Some(id) = create_named(app, otd_gpu::ops::VIDEO_DEVICE_IN, pos, "camera") {
        app.graph.node_mut(id).flags.viewer = true;
        app.document.status =
            "Video Device In — the first frame arrives once the camera permission is granted"
                .into();
    }
}

const ADD_OP_FAMILIES: [otd_core::Family; 6] = [
    otd_core::Family::Top,
    otd_core::Family::Chop,
    otd_core::Family::Sop,
    otd_core::Family::Dat,
    otd_core::Family::Mat,
    otd_core::Family::Comp,
];

fn op_wires_after(def: &otd_core::OpDef, src: otd_core::Family) -> bool {
    if def.inputs.is_empty() {
        return true;
    }
    def.input_families.first().copied().unwrap_or(def.family) == src
}

pub fn add_operator_menu(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    only_family: Option<otd_core::Family>,
) {
    let src_family = app
        .canvas.selected
        .filter(|s| app.graph.contains(*s))
        .map(|s| app.graph.node(s).family);

    type OpEntry<'a> = (&'a str, &'a str, &'a str);
    let by_family: Vec<(otd_core::Family, Vec<OpEntry<'_>>)> = ADD_OP_FAMILIES
        .iter()
        .copied()
        .filter(|family| only_family.is_none_or(|only| only == *family))
        .map(|family| {
            let mut ops: Vec<_> = app
                .registry
                .iter()
                .filter(|d| d.family == family)
                .filter(|d| src_family.is_none_or(|src| op_wires_after(d, src)))
                .map(|d| (d.type_name, d.label, d.summary))
                .collect();
            ops.sort_by_key(|(_, label, _)| label.to_lowercase());
            (family, ops)
        })
        .collect();

    for (family, ops) in &by_family {
        if ops.is_empty() {
            continue;
        }
        let mut list = |ui: &mut egui::Ui| {
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    for (type_name, label, summary) in ops {
                        if ui.button(*label).on_hover_text(*summary).clicked() {
                            app.queue_add_effect(type_name);
                            ui.close();
                        }
                    }
                });
        };
        if only_family.is_some() {
            list(ui);
        } else {
            ui.menu_button(format!("Add {}", family.suffix()), list);
        }
    }

    ui.separator();
    ui.menu_button("Recipes", |ui| {
        for recipe in otd_ai::recipes::all()
            .iter()
            .filter(|r| r.needs == otd_ai::recipes::Needs::Video)
        {
            if ui
                .button(&recipe.name)
                .on_hover_text(&recipe.prompt)
                .clicked()
            {
                crate::assistant::apply_recipe(app, recipe);
                ui.close();
            }
        }
    });
}

pub fn pick_for(op_type: &str, key: &str) -> Option<std::path::PathBuf> {
    if op_type == otd_gpu::ops::MOVIE_OUT {
        return rfd::FileDialog::new()
            .set_title("Record to")
            .add_filter("Movie", MOVIE_EXT)
            .set_file_name("out.mp4")
            .save_file();
    }
    let dialog = rfd::FileDialog::new().set_title("Choose a file");
    let dialog = match (op_type, key) {
        (otd_chop::io::AUDIO_FILE, _) => dialog.add_filter("Audio (WAV)", &["wav"]),
        (_, "font") => dialog.add_filter("Font", &["ttf", "otf"]),
        (otd_gpu::ops::MOVIE_IN, _) => {
            let mut all: Vec<&str> = Vec::new();
            all.extend_from_slice(MOVIE_EXT);
            all.extend_from_slice(IMAGE_EXT);
            dialog.add_filter("Movies and images", &all)
        }
        _ => dialog,
    };
    dialog.add_filter("Anything", &["*"]).pick_file()
}

pub fn replace_file(app: &mut OtdApp, id: NodeId) {
    let op_type = app.graph.node(id).op_type.clone();
    let Some(path) = pick_for(&op_type, "file") else {
        return;
    };
    app.edit("set file");
    app.document.history.end_gesture();
    let stored = stored_path(app, &path);
    let _ = app.graph.set_param(id, "file", Value::Str(stored));
    app.graph.node_mut(id).flags.viewer = true;
    app.document.status = format!(
        "{} -> {}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        app.graph.node(id).name
    );
}