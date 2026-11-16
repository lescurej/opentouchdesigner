
use egui::{Color32, RichText};
use otd_ai::Provider;

use crate::app::OtdApp;

use super::labels::{attachment_row, model_label};
use super::apply::recipes_menu;
use super::patch::{poll, scope_of, send};
pub fn bar(app: &mut OtdApp, ctx: &egui::Context) {
    // Before anything that might return early: a hidden bar still watches.
    poll(app);
    crate::watch::tick(app);
    if app.assistant.watch.is_some() {
        ctx.request_repaint();
    }
    if app.perform_preview_only() {
        return;
    }

    // Cmd/Ctrl+K from anywhere. The one shortcut worth spending, because a
    // prompt box you have to go and find is a prompt box you do not use.
    let summon = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K));
    if summon {
        app.assistant.bar = true;
        app.assistant.collapsed = false;
        app.assistant.focus_bar = true;
    }
    if !app.assistant.bar {
        return;
    }

    if app.assistant.collapsed {
        collapsed_pill(app, ctx);
        return;
    }

    egui::Area::new(egui::Id::new("assistant-bar"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -56.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // The window frame from the active theme, so the bar looks like
            // part of the program rather than pasted onto it.
            egui::Frame::window(ui.style())
                .corner_radius(egui::CornerRadius::same(14))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_width(560.0);
                    bar_contents(app, ui);
                    // Remembered so a file dropped here is attached rather
                    // than built into the network. See `claim_drop`.
                    app.assistant.bar_rect = ui.min_rect().expand(12.0);
                });
        });
}
pub(crate) fn bar_contents(app: &mut OtdApp, ui: &mut egui::Ui) {
    let busy = app.assistant.busy();

    // ---- the prompt line
    //
    // The hint changes when a picture is attached, because what the box is
    // for changes: with a reference, typing nothing is a complete request and
    // whatever you do type is a correction to it.
    let empty = app.graph.children(app.canvas.current).is_empty();
    let hint = match (app.assistant.image.is_some(), empty) {
        (true, _) => "Enter to build this image — or say what to change about it…",
        (false, true) => "Describe a patch — or pick a Recipe to start from…",
        (false, false) => "Describe a patch, and it gets built here…",
    };
    let edit = ui.add(
        egui::TextEdit::multiline(&mut app.assistant.prompt)
            .frame(egui::Frame::NONE)
            .desired_rows(1)
            .desired_width(f32::INFINITY)
            .hint_text(hint),
    );
    if std::mem::take(&mut app.assistant.focus_bar) {
        edit.request_focus();
    }
    // Enter sends, Shift+Enter is a newline — the convention every chat box
    // has trained everyone into.
    let entered =
        edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
    if entered {
        // The newline arrives with the key press; take it back out.
        let trimmed = app.assistant.prompt.trim_end_matches('\n').to_string();
        app.assistant.prompt = trimmed;
    }

    ui.add_space(4.0);

    // ---- the control row
    ui.horizontal(|ui| {
        if ui
            .small_button("⚙")
            .on_hover_text("Providers, API keys and what was skipped")
            .clicked()
        {
            app.assistant.open = true;
        }

        recipes_menu(app, ui);
        attachment_row(app, ui);

        let watching = app.assistant.watch.is_some();
        if ui
            .selectable_label(watching, "Watch")
            .on_hover_text(
                "Turn the camera on, show the assistant what it sees and build an \
                 effect for it — then it looks at the result and fixes it once if \
                 it is black, blown out, frozen or unchanged. Anything typed is \
                 taken as direction (\"make it moody\"). Click again to stop.",
            )
            .clicked()
        {
            crate::watch::toggle(app);
        }

        // The model chip, as in every chat UI: provider and model in one
        // place, because they are one decision.
        let provider = app.assistant.provider;
        let chip = format!("✨ {}", short_model(&app.assistant.model));
        egui::ComboBox::from_id_salt("assistant-bar-model")
            .selected_text(chip)
            .show_ui(ui, |ui| {
                for p in Provider::ALL {
                    let has = app.assistant.ready(*p);
                    ui.label(
                        RichText::new(format!("{} {}", if has { "●" } else { "○" }, p.label()))
                            .small()
                            .weak(),
                    );
                    for model in p.models() {
                        let selected = provider == *p && app.assistant.model == *model;
                        if ui.selectable_label(selected, model_label(model)).clicked() {
                            app.assistant.provider = *p;
                            app.assistant.model = model.to_string();
                        }
                    }
                    ui.separator();
                }
                if ui.button("More settings…").clicked() {
                    app.assistant.open = true;
                }
            });

        // Removal is a mode rather than a phrasing: asking for it in words is
        // how you find out afterwards which operators the model thought were
        // redundant.
        ui.toggle_value(&mut app.assistant.allow_delete, "🗑")
            .on_hover_text(
                "Let the assistant remove operators as well as add them — \
                 needed for \"simplify this\" or \"take the blur out\"",
            );

        // Next to the dustbin because it is the same kind of switch: a
        // standing limit on what the answer is allowed to do, rather than
        // something you phrase differently each time you want it.
        let count = scope_of(app).len();
        ui.add_enabled_ui(count > 0 || app.assistant.scope_selection, |ui| {
            ui.toggle_value(&mut app.assistant.scope_selection, "🔒")
                .on_hover_text(match count {
                    0 => "Confine the request to the selected operators — \
                          but nothing is selected, so this would do nothing"
                        .to_string(),
                    1 => "Confine the request to the 1 selected operator — \
                          the rest of the network is left exactly as it is"
                        .to_string(),
                    n => format!(
                        "Confine the request to the {n} selected operators — \
                         the rest of the network is left exactly as it is"
                    ),
                });
        });

        if app.assistant.not_configured() {
            let (short, hint) = match app.assistant.provider.needs_key() {
                true => ("no key", "Click ⚙ to paste an API key"),
                false => (
                    "not installed",
                    "Click ⚙ — this provider runs a CLI that is not on this machine",
                ),
            };
            ui.label(
                RichText::new(short)
                    .small()
                    .color(Color32::from_rgb(235, 170, 90)),
            )
            .on_hover_text(hint);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button("×")
                .on_hover_text("Hide the bar (Cmd/Ctrl+K brings it back)")
                .clicked()
            {
                app.assistant.bar = false;
            }
            if ui
                .small_button("–")
                .on_hover_text("Collapse (Escape)")
                .clicked()
            {
                app.assistant.collapsed = true;
            }
            if busy {
                ui.spinner();
            } else {
                let ready = app.assistant.has_request();
                if ui
                    .add_enabled(ready, egui::Button::new("▶"))
                    .on_hover_text("Build it (Enter)")
                    .clicked()
                    || (entered && ready)
                {
                    send(app);
                }
            }
        });
    });

    // ---- one line of what happened, so the bar answers on its own
    if let Some(watch) = &app.assistant.watch {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new(watch.label()).small().weak());
        });
    } else if let Some(error) = &app.assistant.error {
        ui.label(
            RichText::new(one_line(error))
                .small()
                .color(Color32::from_rgb(235, 120, 120)),
        )
        .on_hover_text(error);
    } else if !app.assistant.broken.is_empty() {
        // The model's own notes describe the patch it meant to build, which
        // is not the one on the canvas. A red node beats a cheerful sentence.
        let names: Vec<&str> = app
            .assistant
            .broken
            .iter()
            .map(|(node, _)| node.as_str())
            .collect();
        let detail = app
            .assistant
            .broken
            .iter()
            .map(|(node, error)| format!("{node}: {error}"))
            .collect::<Vec<_>>()
            .join("\n");
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} will not compile — black from there down",
                    names.join(", ")
                ))
                .small()
                .color(Color32::from_rgb(235, 120, 120)),
            )
            .on_hover_text(detail);
            if ui.small_button("⚠").clicked() {
                app.assistant.open = true;
            }
        });
    } else if let Some(notes) = &app.assistant.last {
        ui.horizontal(|ui| {
            ui.label(RichText::new(one_line(notes)).small().weak())
                .on_hover_text(notes);
            if !app.assistant.warnings.is_empty() && ui.small_button("⚠").clicked() {
                app.assistant.open = true;
            }
        });
    }

    // Escape collapses rather than closing: the work you typed survives.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) && !app.assistant.prompt.is_empty() {
        app.assistant.collapsed = true;
    }
}
pub(crate) fn collapsed_pill(app: &mut OtdApp, ctx: &egui::Context) {
    egui::Area::new(egui::Id::new("assistant-pill"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -56.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::window(ui.style())
                .corner_radius(egui::CornerRadius::same(14))
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if app.assistant.busy() {
                            ui.spinner();
                        }
                        if ui
                            .button("✨ Assistant")
                            .on_hover_text("Cmd/Ctrl+K")
                            .clicked()
                        {
                            app.assistant.collapsed = false;
                            app.assistant.focus_bar = true;
                        }
                    });
                });
        });
}
pub(crate) fn short_model(model: &str) -> String {
    let tail = model.rsplit('/').next().unwrap_or(model);
    let trimmed = match tail.rsplit_once('-') {
        // Drop a trailing 8-digit date, keep everything else.
        Some((head, last)) if last.len() == 8 && last.chars().all(|c| c.is_ascii_digit()) => head,
        _ => tail,
    };
    if trimmed.chars().count() > 22 {
        format!("{}…", trimmed.chars().take(21).collect::<String>())
    } else {
        trimmed.to_string()
    }
}
pub(crate) fn one_line(text: &str) -> String {
    let first = text.split(['\n', '.']).next().unwrap_or(text).trim();
    if first.chars().count() > 90 {
        format!("{}…", first.chars().take(89).collect::<String>())
    } else {
        first.to_string()
    }
}
