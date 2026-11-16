use egui::{
    Align, Align2, Color32, FontId, Frame, Key, Layout, Margin, Modifiers, Popup, Sense,
    Stroke, TextStyle, Ui, Vec2,
};
use egui_extras::syntax_highlighting::{CodeTheme, highlight_with};

use super::completion::{completion_caret_rect, completion_context, completion_popup};
use super::paint::{
    code_pixel_width, indent_guide_columns, paint_indent_guides, paint_issue_squiggles,
};
use super::util::{
    byte_cursor, dat_text_edit_id, popup_was_open, set_text_edit_cursor, syntect_language, SYNTECT,
};
use super::{CodeEditorConfig, CodeEditorExtras};

pub fn show(
    ui: &mut Ui,
    text: &mut String,
    config: CodeEditorConfig,
    extras: Option<CodeEditorExtras<'_>>,
) -> egui::Response {
    let theme = CodeTheme::from_style(ui.style());
    let lang = syntect_language(config.language);
    let bg = if theme.is_dark() {
        Color32::from_rgb(30, 30, 33)
    } else {
        Color32::from_rgb(250, 250, 252)
    };
    let gutter_bg = if theme.is_dark() {
        Color32::from_rgb(24, 24, 27)
    } else {
        Color32::from_rgb(245, 245, 248)
    };
    let border = Color32::from_gray(if theme.is_dark() { 55 } else { 190 });
    let gutter_line = Color32::from_gray(if theme.is_dark() { 110 } else { 150 });

    let font_id = ui
        .style()
        .text_styles
        .get(&TextStyle::Monospace)
        .cloned()
        .unwrap_or_else(|| FontId::monospace(13.0));
    let row_h = ui.fonts_mut(|f| f.row_height(&font_id));
    let char_w = ui.fonts_mut(|f| f.glyph_width(&font_id, 'm'));

    let line_count = text.lines().count().max(1);
    let digits = (line_count as f32).log10().floor() as usize + 1;
    let gutter_w = 14.0 + digits as f32 * char_w;
    let viewport_h = row_h * config.rows as f32 + 16.0;

    const EDITOR_MARGIN: Margin = Margin {
        left: 10,
        right: 12,
        top: 4,
        bottom: 4,
    };
    let content_h = row_h * line_count as f32;
    let block_h = content_h + EDITOR_MARGIN.sum().y;

    let issues = extras.as_ref().map(|e| e.issues).unwrap_or(&[]);
    let error_lines: std::collections::HashSet<u32> = issues
        .iter()
        .filter(|i| i.severity == otd_engine::LintSeverity::Error)
        .map(|i| i.line)
        .collect();
    let warn_lines: std::collections::HashSet<u32> = issues
        .iter()
        .filter(|i| i.severity == otd_engine::LintSeverity::Warning)
        .map(|i| i.line)
        .collect();

    let mut changed = false;
    let mut outer = ui.allocate_response(Vec2::ZERO, Sense::hover());
    let owner = config.id;
    let popup_id = owner.with("complete_popup");
    let completion_enabled = config.interactive && extras.is_some();

    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .inner_margin(0.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::ScrollArea::both()
                .id_salt(config.id)
                .max_height(viewport_h)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let code_w =
                        code_pixel_width(text, char_w).max(ui.available_width() - gutter_w);
                    let block_w = (gutter_w + code_w).max(ui.available_width());
                    let (block_rect, _) =
                        ui.allocate_exact_size(Vec2::new(block_w, block_h), Sense::hover());

                    let gutter_rect = egui::Rect::from_min_max(
                        block_rect.min,
                        egui::pos2(block_rect.min.x + gutter_w, block_rect.max.y),
                    );
                    let editor_rect = egui::Rect::from_min_max(
                        egui::pos2(block_rect.min.x + gutter_w, block_rect.min.y),
                        block_rect.max,
                    );
                    let text_rect = egui::Rect::from_min_max(
                        egui::pos2(
                            editor_rect.min.x + EDITOR_MARGIN.left as f32,
                            editor_rect.min.y + EDITOR_MARGIN.top as f32,
                        ),
                        egui::pos2(
                            editor_rect.max.x - EDITOR_MARGIN.right as f32,
                            editor_rect.max.y - EDITOR_MARGIN.bottom as f32,
                        ),
                    );

                    let painter = ui.painter().with_clip_rect(block_rect);
                    painter.rect_filled(gutter_rect, 0.0, gutter_bg);
                    painter.rect_filled(editor_rect, 0.0, bg);
                    painter.vline(
                        gutter_rect.right(),
                        block_rect.y_range(),
                        Stroke::new(1.0, border),
                    );

                    let guide_cols = indent_guide_columns(text);
                    paint_indent_guides(&painter, text_rect, char_w, &guide_cols);

                    paint_issue_squiggles(&painter, text_rect, row_h, char_w, issues, text);

                    for i in 1..=line_count {
                        let y = text_rect.min.y + (i - 1) as f32 * row_h;
                        let line_u = i as u32;
                        let num_color = if error_lines.contains(&line_u) {
                            Color32::from_rgb(230, 110, 110)
                        } else if warn_lines.contains(&line_u) {
                            Color32::from_rgb(230, 190, 110)
                        } else {
                            gutter_line
                        };
                        painter.text(
                            egui::pos2(gutter_rect.right() - 8.0, y),
                            Align2::RIGHT_TOP,
                            format!("{i:>digits$}"),
                            font_id.clone(),
                            num_color,
                        );
                    }

                    let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
                        let mut job = highlight_with(
                            ui.ctx(),
                            ui.style(),
                            &theme,
                            buf.as_str(),
                            lang,
                            &SYNTECT,
                        );
                        job.wrap.max_width = f32::INFINITY;
                        ui.fonts_mut(|f| f.layout_job(job))
                    };

                    let edit_id = dat_text_edit_id(owner);
                    let editor = egui::TextEdit::multiline(text)
                        .id(edit_id)
                        .font(TextStyle::Monospace)
                        .code_editor()
                        .interactive(config.interactive)
                        .frame(Frame::NONE)
                        .margin(Margin::ZERO)
                        .background_color(Color32::TRANSPARENT)
                        .desired_width(text_rect.width())
                        .desired_rows(line_count)
                        .layouter(&mut layouter);
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(text_rect)
                            .layout(Layout::top_down(Align::LEFT)),
                    );
                    let output = editor.show(&mut child);
                    let response = output.response.response.clone();
                    let mut dismiss_completion = false;
                    if completion_enabled
                        && (response.has_focus() || popup_was_open(ui.ctx(), popup_id))
                    {
                        ui.ctx().input_mut(|i| {
                            if i.consume_key(Modifiers::NONE, Key::Escape) {
                                dismiss_completion = true;
                            }
                        });
                    }
                    let cursor = byte_cursor(text, &output.state);
                    let caret = completion_caret_rect(&output, row_h);
                    output.state.store(ui.ctx(), response.id);
                    if response.changed() {
                        changed = true;
                    }
                    outer = response.clone();

                    if completion_enabled {
                        if dismiss_completion {
                            Popup::close_id(ui.ctx(), popup_id);
                        }
                        if let Some(ex) = extras.as_ref() {
                            let kind = completion_context(text, cursor, ex.me_path);
                            let ctx_key = popup_id.with("in_ctx");
                            let in_ctx = kind.is_some();
                            let was_in = ui
                                .ctx()
                                .data(|d| d.get_temp::<bool>(ctx_key).unwrap_or(false));
                            ui.ctx().data_mut(|d| d.insert_temp(ctx_key, in_ctx));

                            let open_cmd = if dismiss_completion || !in_ctx {
                                Some(false)
                            } else if !was_in {
                                Some(true)
                            } else {
                                None
                            };

                            if let Some(kind) = kind {
                                if let Some(after_byte) = completion_popup(
                                    ui.ctx(),
                                    popup_id,
                                    open_cmd,
                                    kind,
                                    text,
                                    cursor,
                                    ex,
                                    caret,
                                    response.layer_id,
                                ) {
                                    changed = true;
                                    set_text_edit_cursor(ui.ctx(), response.id, text, after_byte);
                                }
                            }
                        }
                    }
                });
        });

    if changed {
        outer.mark_changed();
    }
    outer
}