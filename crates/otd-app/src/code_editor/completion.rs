use egui::containers::menu::menu_style;
use egui::text_selection::text_cursor_state::cursor_rect;
use egui::widgets::text_edit::TextEditOutput;
use egui::{
    Align, Color32, Context, Id, LayerId, Layout, Popup, PopupKind, Rect, RichText, ScrollArea,
    Sense,
};

use super::CodeEditorExtras;

pub(crate) enum CompletionKind {
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

pub(crate) fn completion_context(text: &str, cursor_byte: usize, _me_path: &str) -> Option<CompletionKind> {
    if let Some((replace_start, replace_end, quote, prefix)) =
        call_open_string(text, cursor_byte, "op(")
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

pub(crate) fn completion_caret_rect(output: &TextEditOutput, row_h: f32) -> Rect {
    let Some(range) = output.cursor_range else {
        return output.response.response.rect;
    };
    let local = cursor_rect(&output.galley, &range.primary, row_h);
    let offset = output.galley_pos.to_vec2() - egui::vec2(output.galley.rect.left(), 0.0);
    local.translate(offset)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn completion_popup(
    ctx: &Context,
    popup_id: Id,
    open_cmd: Option<bool>,
    kind: CompletionKind,
    text: &mut String,
    cursor: usize,
    ex: &CodeEditorExtras<'_>,
    caret: Rect,
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
                        cursor_after_pick = Some(replace_start + insert.len() + quote.len_utf8());
                        *text = new;
                        ui.close();
                    }
                }
            });
        });
    cursor_after_pick
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