use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, RichText, Sense, Stroke, StrokeKind, Vec2,
};
use otd_core::Family;

use crate::app::OtdApp;

const TABS: [Family; 6] = [
    Family::Comp,
    Family::Top,
    Family::Chop,
    Family::Sop,
    Family::Mat,
    Family::Dat,
];

const COLS: usize = 6;

fn search_id() -> egui::Id {
    egui::Id::new("op_create_dialog_search")
}
const CELL_W: f32 = 110.0;
const CELL_H: f32 = 22.0;

fn default_pos(ctx: &egui::Context) -> Pos2 {
    let r = ctx.content_rect();
    Pos2::new(r.center().x - 340.0, r.top() + 56.0)
}

fn op_matches(type_name: &str, label: &str, filter: &str) -> bool {
    filter.is_empty()
        || type_name.to_lowercase().contains(filter)
        || label.to_lowercase().contains(filter)
}

fn tab_color(family: Family) -> Color32 {
    match family {
        Family::Comp => Color32::from_rgb(78, 82, 92),
        Family::Top => Color32::from_rgb(92, 68, 128),
        Family::Chop => Color32::from_rgb(58, 128, 78),
        Family::Sop => Color32::from_rgb(52, 108, 118),
        Family::Mat => Color32::from_rgb(128, 102, 52),
        Family::Dat => Color32::from_rgb(128, 58, 98),
    }
}

pub fn show(app: &mut OtdApp, ui: &egui::Ui) {
    let Some(dialog) = &mut app.create_dialog else {
        return;
    };
    let mut open = true;
    let mut close_requested = false;
    let mut chosen: Option<String> = None;
    let filter = dialog.filter.to_lowercase();
    let active = dialog.family;
    let world_pos = dialog.world_pos;
    let connect_from = dialog.connect_from;

    let window = egui::Window::new("OP Create Dialog")
        .id(egui::Id::new("op_create_dialog"))
        .collapsible(false)
        .resizable(false)
        .movable(true)
        .open(&mut open)
        .default_pos(default_pos(ui.ctx()))
        .default_width(680.0)
        .show(ui.ctx(), |ui| {
            ui.set_width(680.0);
            let want_focus = app.create_dialog.as_mut().unwrap().focus;
            let edit = ui.add(
                egui::TextEdit::singleline(&mut app.create_dialog.as_mut().unwrap().filter)
                    .id(search_id())
                    .hint_text("Search")
                    .desired_width(f32::INFINITY),
            );
            if want_focus {
                edit.request_focus();
                ui.ctx().memory_mut(|mem| mem.request_focus(search_id()));
                if edit.has_focus() {
                    app.create_dialog.as_mut().unwrap().focus = false;
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for family in TABS {
                    let selected = active == family;
                    let base = tab_color(family);
                    let fill = if selected {
                        base
                    } else {
                        base.gamma_multiply(0.55)
                    };
                    let label = RichText::new(family.suffix())
                        .color(Color32::WHITE)
                        .size(13.0);
                    let stroke = if selected {
                        Stroke::new(1.0, Color32::from_gray(220))
                    } else {
                        Stroke::NONE
                    };
                    if ui
                        .add(
                            egui::Button::new(label)
                                .fill(fill)
                                .stroke(stroke)
                                .corner_radius(CornerRadius::ZERO)
                                .min_size(Vec2::new(72.0, 24.0)),
                        )
                        .clicked()
                    {
                        app.create_dialog.as_mut().unwrap().family = family;
                    }
                }
            });

            let mut entries: Vec<(&str, &str, &str, bool)> = app
                .registry
                .iter()
                .filter(|d| d.family == active)
                .map(|d| {
                    let hit = op_matches(d.type_name, d.label, &filter);
                    (d.type_name, d.label, d.summary, hit)
                })
                .collect();
            entries.sort_by_key(|(_, label, _, _)| label.to_lowercase());

            let list_bg = tab_color(active).gamma_multiply(0.72);
            let grid_line = tab_color(active).gamma_multiply(1.0);
            let cell_stroke = Stroke::new(1.0, grid_line);
            let stripe_b = list_bg.gamma_multiply(0.86);
            egui::Frame::NONE
                .fill(list_bg)
                .inner_margin(0.0)
                .show(ui, |ui| {
                    let rows = entries.len().div_ceil(COLS).max(1);
                    egui::ScrollArea::vertical()
                        .max_height(360.0)
                        .show(ui, |ui| {
                            ui.set_min_width(COLS as f32 * CELL_W);
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            for row in 0..rows {
                                let row_bg = if row % 2 == 0 {
                                    list_bg
                                } else {
                                    stripe_b
                                };
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    for col in 0..COLS {
                                        let idx = row + col * rows;
                                        let sense = entries
                                            .get(idx)
                                            .map(|(_, _, _, hit)| {
                                                if *hit {
                                                    Sense::click()
                                                } else {
                                                    Sense::hover()
                                                }
                                            })
                                            .unwrap_or(Sense::hover());
                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(CELL_W, CELL_H),
                                            sense,
                                        );
                                        let mut fill = row_bg;
                                        if let Some((_, _, _, hit)) = entries.get(idx) {
                                            if *hit && resp.hovered() {
                                                fill = tab_color(active).gamma_multiply(1.08);
                                            }
                                        }
                                        let painter = ui.painter();
                                        painter.rect_filled(rect, CornerRadius::ZERO, fill);
                                        painter.rect_stroke(
                                            rect,
                                            CornerRadius::ZERO,
                                            cell_stroke,
                                            StrokeKind::Inside,
                                        );
                                        if let Some((type_name, label, summary, hit)) =
                                            entries.get(idx)
                                        {
                                            let label_color = if *hit {
                                                if resp.hovered() {
                                                    Color32::WHITE
                                                } else {
                                                    Color32::from_gray(235)
                                                }
                                            } else {
                                                Color32::from_gray(140)
                                            };
                                            painter.text(
                                                rect.left_center() + Vec2::new(6.0, 0.0),
                                                Align2::LEFT_CENTER,
                                                *label,
                                                FontId::proportional(13.0),
                                                label_color,
                                            );
                                            if *hit && resp.clicked() {
                                                chosen = Some(type_name.to_string());
                                            }
                                            resp.on_hover_text(*summary);
                                        }
                                    }
                                });
                            }
                        });
                });

            if ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some((type_name, ..)) = entries.iter().find(|(_, _, _, hit)| *hit) {
                    chosen = Some(type_name.to_string());
                }
            }
            if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                close_requested = true;
            }
        });
    app.create_dialog_rect = window.as_ref().map(|r| r.response.rect);
    let open = open && !close_requested;

    if let Some(type_name) = chosen {
        if let Some(new_id) = app.create_node(&type_name, world_pos) {
            if let Some(src) = connect_from.filter(|s| app.graph.contains(*s)) {
                let _ = app.graph.connect(src, new_id, 0);
            }
        }
        app.create_dialog = None;
        app.create_dialog_rect = None;
    } else if !open {
        app.create_dialog = None;
        app.create_dialog_rect = None;
    }
}
