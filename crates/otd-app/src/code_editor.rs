use std::sync::LazyLock;

use egui::{
    Align, Align2, Color32, FontId, Frame, Id, Key, LayerId, Layout, Margin, Modifiers, Popup,
    PopupKind, RichText, ScrollArea, Sense, Stroke, TextStyle, Ui, Vec2,
};
use egui::containers::menu::menu_style;
use egui::text::{CCursor, CCursorRange};
use egui::widgets::text_edit::{TextEditOutput, TextEditState};
use egui_extras::syntax_highlighting::{CodeTheme, SyntectSettings, highlight_with};
use otd_core::Graph;
use otd_engine::Engines;
use otd_engine::PythonLint;

static SYNTECT: LazyLock<SyntectSettings> = LazyLock::new(SyntectSettings::default);

#[derive(Clone, Copy)]
pub struct CodeEditorConfig {
    pub language: &'static str,
    pub rows: usize,
    pub id: Id,
    pub interactive: bool,
}

pub struct CodeEditorExtras<'a> {
    pub issues: &'a [PythonLint],
    pub op_paths: &'a [String],
    pub me_path: &'a str,
    pub graph: &'a Graph,
    pub engines: &'a Engines,
}

pub fn dat_editor_id(id: otd_core::NodeId) -> Id {
    Id::new(("code_editor", id))
}

pub fn dat_node_viewer_editor_id(id: otd_core::NodeId) -> Id {
    Id::new(("code_editor", "node_viewer", id))
}

fn dat_text_edit_id(owner: Id) -> Id {
    owner.with("edit")
}

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
    let block_h = content_h + EDITOR_MARGIN.sum().y as f32;

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
            ScrollArea::both()
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
                    paint_indent_guides(
                        &painter,
                        text_rect,
                        char_w,
                        &guide_cols,
                    );

                    paint_issue_squiggles(
                        &painter,
                        text_rect,
                        row_h,
                        char_w,
                        issues,
                        text,
                    );

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

                    let mut layouter =
                        |ui: &Ui, buf: &dyn egui::TextBuffer, _wrap_width: f32| {
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

                            let open_cmd = if dismiss_completion {
                                Some(false)
                            } else if !in_ctx {
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

fn popup_was_open(ctx: &egui::Context, popup_id: Id) -> bool {
    ctx.read_response(popup_id).is_some()
}

fn syntect_language<'a>(name: &'a str) -> &'a str {
    match name {
        "python" | "py" => "Python",
        "text" | "txt" => "Plain Text",
        "rust" | "rs" => "Rust",
        "javascript" | "js" => "JavaScript",
        "typescript" | "ts" => "TypeScript",
        "glsl" => "GLSL",
        "wgsl" => "WGSL",
        _ => name,
    }
}

fn code_pixel_width(text: &str, char_w: f32) -> f32 {
    let max_chars = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1);
    max_chars as f32 * char_w + 32.0
}

fn indent_guide_columns(text: &str) -> Vec<usize> {
    let tab = 4;
    let mut cols = std::collections::BTreeSet::new();
    for line in text.lines() {
        let mut col = 0usize;
        for c in line.chars() {
            match c {
                ' ' => col += 1,
                '\t' => col += tab,
                _ => break,
            }
        }
        let mut t = tab;
        while t < col {
            cols.insert(t);
            t += tab;
        }
    }
    cols.into_iter().collect()
}

fn paint_indent_guides(
    painter: &egui::Painter,
    rect: egui::Rect,
    char_w: f32,
    columns: &[usize],
) {
    if columns.is_empty() {
        return;
    }
    let color = Color32::from_rgba_unmultiplied(90, 100, 120, 100);
    let stroke = Stroke::new(1.0, color);
    let clip = painter.with_clip_rect(rect);
    for &col in columns {
        let x = rect.min.x + col as f32 * char_w;
        if x >= rect.max.x {
            break;
        }
        clip.line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            stroke,
        );
    }
}

pub fn is_callback_dat_text(app: &crate::app::OtdApp, id: otd_core::NodeId) -> bool {
    app.graph.node(id).callback_owner.is_some()
        && app.graph.node(id).op_type == otd_dat::ops::TEXT
}

fn paint_issue_squiggles(
    painter: &egui::Painter,
    text_rect: egui::Rect,
    row_h: f32,
    char_w: f32,
    issues: &[PythonLint],
    text: &str,
) {
    for issue in issues {
        let line_idx = issue.line as usize;
        if line_idx == 0 {
            continue;
        }
        let Some(line_text) = text.lines().nth(line_idx - 1) else {
            continue;
        };
        let y = text_rect.min.y + (line_idx - 1) as f32 * row_h + row_h - 2.0;
        let x0 = text_rect.min.x + issue.column as f32 * char_w;
        let span = if issue.span_len > 0 {
            issue.span_len as f32 * char_w
        } else {
            line_text.chars().count().max(1) as f32 * char_w
        };
        let x1 = (x0 + span).min(text_rect.max.x);
        if x1 <= x0 {
            continue;
        }
        let color = match issue.severity {
            otd_engine::LintSeverity::Error => Color32::from_rgb(230, 110, 110),
            otd_engine::LintSeverity::Warning => Color32::from_rgb(230, 190, 110),
        };
        paint_wavy_line(painter, x0, x1, y, color);
    }
}

fn paint_wavy_line(painter: &egui::Painter, x0: f32, x1: f32, y: f32, color: Color32) {
    let stroke = Stroke::new(1.0, color);
    let amp = 1.5;
    let wave = 4.0;
    let mut x = x0;
    let mut up = true;
    while x < x1 {
        let nx = (x + wave).min(x1);
        let y1 = if up { y - amp } else { y + amp };
        painter.line_segment([egui::pos2(x, y), egui::pos2(nx, y1)], stroke);
        x = nx;
        up = !up;
    }
}

enum CompletionKind {
    OpPath {
        replace_start: usize,
        replace_end: usize,
        quote: char,
        prefix: String,
    },
    ChOpPath {
        replace_start: usize,
        replace_end: usize,
        quote: char,
        prefix: String,
    },
    ChChannel {
        replace_start: usize,
        replace_end: usize,
        quote: char,
        prefix: String,
    },
}

fn completion_context(text: &str, cursor_byte: usize, _me_path: &str) -> Option<CompletionKind> {
    if let Some((replace_start, replace_end, quote, prefix)) = call_open_string(text, cursor_byte, "op(")
    {
        return Some(CompletionKind::OpPath {
            replace_start,
            replace_end,
            quote,
            prefix,
        });
    }
    if let Some(ch) = ch_completion_context(text, cursor_byte) {
        return Some(ch);
    }
    None
}

fn ch_completion_context(text: &str, cursor_byte: usize) -> Option<CompletionKind> {
    let head = &text[..cursor_byte.min(text.len())];
    let ch_pos = head.rfind("ch(")?;
    let mut j = ch_pos + 3;
    skip_ws(text, &mut j);
    if j >= cursor_byte {
        return None;
    }
    let q1 = *text.as_bytes().get(j)?;
    if q1 != b'\'' && q1 != b'"' {
        return None;
    }
    let q1c = q1 as char;
    j += 1;
    let content1_start = j;
    while j < cursor_byte {
        if text.as_bytes()[j] == q1 {
            break;
        }
        if text.as_bytes()[j] == b'\\' {
            j = (j + 2).min(cursor_byte);
        } else {
            j += 1;
        }
    }
    if j >= cursor_byte {
        return Some(CompletionKind::ChOpPath {
            replace_start: content1_start,
            replace_end: cursor_byte,
            quote: q1c,
            prefix: text[content1_start..cursor_byte].to_string(),
        });
    }
    j += 1;
    skip_ws(text, &mut j);
    if text.as_bytes().get(j) != Some(&b',') {
        return None;
    }
    j += 1;
    skip_ws(text, &mut j);
    if j >= cursor_byte {
        return None;
    }
    let q2 = *text.as_bytes().get(j)?;
    if q2 != b'\'' && q2 != b'"' {
        return None;
    }
    let q2c = q2 as char;
    j += 1;
    let content2_start = j;
    if cursor_byte <= content2_start {
        return None;
    }
    let partial = &text[content2_start..cursor_byte];
    if partial.contains(q2c) {
        return None;
    }
    Some(CompletionKind::ChChannel {
        replace_start: content2_start,
        replace_end: cursor_byte,
        quote: q2c,
        prefix: partial.to_string(),
    })
}

fn skip_ws(text: &str, j: &mut usize) {
    while *j < text.len() && text.as_bytes()[*j].is_ascii_whitespace() {
        *j += 1;
    }
}

fn call_open_string(
    text: &str,
    cursor_byte: usize,
    func: &str,
) -> Option<(usize, usize, char, String)> {
    let head = &text[..cursor_byte.min(text.len())];
    let call_pos = head.rfind(func)?;
    let tail = &text[call_pos + func.len()..cursor_byte];
    let mut i = 0usize;
    skip_ws_bytes(tail, &mut i);
    let quote = tail.as_bytes().get(i).copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    let q = quote as char;
    let after_quote = &tail[i + 1..];
    if after_quote.contains(q) {
        return None;
    }
    let replace_start = call_pos + func.len() + i + 1;
    Some((replace_start, cursor_byte, q, after_quote.to_string()))
}

fn skip_ws_bytes(s: &str, i: &mut usize) {
    while *i < s.len() && s.as_bytes()[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn completion_caret_rect(output: &TextEditOutput, row_h: f32) -> egui::Rect {
    use egui::text_selection::text_cursor_state::cursor_rect;
    let Some(range) = output.cursor_range else {
        return output.response.response.rect;
    };
    let local = cursor_rect(&output.galley, &range.primary, row_h);
    let offset = output.galley_pos.to_vec2() - egui::vec2(output.galley.rect.left(), 0.0);
    local.translate(offset)
}

fn completion_popup(
    ctx: &egui::Context,
    popup_id: Id,
    open_cmd: Option<bool>,
    kind: CompletionKind,
    text: &mut String,
    cursor: usize,
    ex: &CodeEditorExtras<'_>,
    caret: egui::Rect,
    layer_id: LayerId,
) -> Option<usize> {
    let (replace_start, replace_end, quote, _prefix, title, items): (
        usize,
        usize,
        char,
        String,
        &str,
        Vec<String>,
    ) = match &kind {
        CompletionKind::OpPath {
            replace_start,
            replace_end,
            quote,
            prefix,
        } => (
            *replace_start,
            *replace_end,
            *quote,
            prefix.clone(),
            "Operator for op('…')",
            filter_op_paths(ex.op_paths, prefix),
        ),
        CompletionKind::ChOpPath {
            replace_start,
            replace_end,
            quote,
            prefix,
        } => (
            *replace_start,
            *replace_end,
            *quote,
            prefix.clone(),
            "CHOP for ch('…', '…')",
            filter_op_paths(ex.op_paths, prefix),
        ),
        CompletionKind::ChChannel {
            replace_start,
            replace_end,
            quote,
            prefix,
        } => {
            let op_ref = ch_op_ref_at(text, cursor).unwrap_or_default();
            let names =
                crate::code_lint::channel_names_at(ex.engines, ex.graph, ex.me_path, &op_ref);
            (
                *replace_start,
                *replace_end,
                *quote,
                prefix.clone(),
                "Channel for ch('…', '…')",
                filter_strings(&names, prefix),
            )
        }
    };

    let use_short_names = matches!(
        kind,
        CompletionKind::OpPath { .. } | CompletionKind::ChOpPath { .. }
    );
    let mut cursor_after_pick: Option<usize> = None;
    let anchor = ctx
        .layer_transform_to_global(layer_id)
        .map(|t| t * caret)
        .unwrap_or(caret);
    Popup::new(popup_id, ctx.clone(), anchor, layer_id)
        .open_memory(open_cmd.map(Into::into))
        .kind(PopupKind::Menu)
        .layout(Layout::top_down_justified(Align::LEFT))
        .style(menu_style)
        .show(|ui| {
            ui.set_min_width(240.0);
            ui.label(RichText::new(title).small().weak());
            ui.separator();
            if items.is_empty() {
                ui.label(RichText::new("No matches").weak());
                return;
            }
            ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for item in items {
                    let row = ui.add(
                        egui::Button::new(item.as_str())
                            .sense(Sense::CLICK)
                            .fill(Color32::TRANSPARENT),
                    );
                    if row.clicked() {
                        let path = if use_short_names {
                            item.rsplit('/').next().unwrap_or(item.as_str()).to_string()
                        } else {
                            item.clone()
                        };
                        let mut new = String::new();
                        let insert = path;
                        new.push_str(&text[..replace_start]);
                        new.push_str(&insert);
                        new.push(quote);
                        new.push_str(&text[replace_end..]);
                        cursor_after_pick =
                            Some(replace_start + insert.len() + quote.len_utf8());
                        *text = new;
                        ui.close();
                    }
                }
            });
        });
    cursor_after_pick
}

fn set_text_edit_cursor(ctx: &egui::Context, id: Id, text: &str, byte_pos: usize) {
    let byte_pos = byte_pos.min(text.len());
    if !text.is_char_boundary(byte_pos) {
        return;
    }
    let char_idx = text[..byte_pos].chars().count();
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(char_idx))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

fn ch_op_ref_at(text: &str, cursor: usize) -> Option<String> {
    let head = &text[..cursor.min(text.len())];
    let ch_pos = head.rfind("ch(")?;
    let tail = &text[ch_pos + 3..cursor];
    let mut i = 0;
    skip_ws_bytes(tail, &mut i);
    let q = tail.as_bytes().get(i)?;
    if *q != b'\'' && *q != b'"' {
        return None;
    }
    let qc = *q as char;
    let body = &tail[i + 1..];
    let end = body.find(qc)?;
    Some(body[..end].to_string())
}

fn filter_op_paths(paths: &[String], prefix: &str) -> Vec<String> {
    let needle = prefix.to_ascii_lowercase();
    let mut matches: Vec<String> = paths
        .iter()
        .filter(|p| {
            if needle.is_empty() {
                return true;
            }
            let short = p.rsplit('/').next().unwrap_or(p.as_str());
            short.to_ascii_lowercase().starts_with(&needle)
                || p.to_ascii_lowercase().contains(&needle)
        })
        .take(48)
        .cloned()
        .collect();
    matches.sort();
    matches
}

fn filter_strings(names: &[String], prefix: &str) -> Vec<String> {
    let needle = prefix.to_ascii_lowercase();
    let mut out: Vec<String> = names
        .iter()
        .filter(|n| {
            if needle.is_empty() {
                return true;
            }
            n.to_ascii_lowercase().starts_with(&needle)
        })
        .take(64)
        .cloned()
        .collect();
    out.sort();
    out
}

fn byte_cursor(text: &str, state: &TextEditState) -> usize {
    let ci: usize = state
        .cursor
        .char_range()
        .map(|r| r.primary.index.0)
        .unwrap_or(0);
    text.char_indices()
        .nth(ci)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}
