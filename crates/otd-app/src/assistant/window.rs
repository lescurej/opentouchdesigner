
use egui::{Color32, RichText};
use otd_ai::Provider;

use crate::app::OtdApp;

use super::labels::{attachment_row, model_label, suggestions};
use super::patch::{poll, send};
pub fn window(app: &mut OtdApp, ctx: &egui::Context) {
    if !app.assistant.open {
        return;
    }
    let mut open = true;
    egui::Window::new("Assistant")
        .open(&mut open)
        .default_width(420.0)
        .resizable(true)
        .show(ctx, |ui| body(app, ui));
    if !open {
        app.assistant.open = false;
    }
    // A request in flight has to keep the UI repainting, or the reply lands
    // in a window that is not being drawn.
    if app.assistant.busy() {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
pub(crate) fn body(app: &mut OtdApp, ui: &mut egui::Ui) {
    poll(app);

    // ---- provider and model
    ui.horizontal(|ui| {
        ui.label("Provider");
        let current = app.assistant.provider;
        egui::ComboBox::from_id_salt("ai-provider")
            .selected_text(current.label())
            .show_ui(ui, |ui| {
                for provider in Provider::ALL {
                    let mark = if app.assistant.ready(*provider) {
                        "●"
                    } else {
                        "○"
                    };
                    if ui
                        .selectable_label(
                            current == *provider,
                            format!("{mark} {}", provider.label()),
                        )
                        .clicked()
                        && current != *provider
                    {
                        app.assistant.provider = *provider;
                        app.assistant.model = provider.default_model().to_string();
                        app.assistant.key_input.clear();
                        app.assistant.error = None;
                    }
                }
            });
        ui.label(RichText::new("● ready to use").weak().small());
    });

    ui.horizontal(|ui| {
        ui.label("Model");
        let provider = app.assistant.provider;
        egui::ComboBox::from_id_salt("ai-model")
            .selected_text(model_label(&app.assistant.model).to_string())
            .show_ui(ui, |ui| {
                for model in provider.models() {
                    if ui
                        .selectable_label(app.assistant.model == *model, model_label(model))
                        .clicked()
                    {
                        app.assistant.model = model.to_string();
                    }
                }
            });
        // Free text as well as the list: the list will be out of date before
        // anybody reads it, and a model name is not a closed set.
        ui.add(
            egui::TextEdit::singleline(&mut app.assistant.model)
                .desired_width(f32::INFINITY)
                .hint_text("or type a model name"),
        );
    });

    // ---- key, or the CLI that stands in for one
    let provider = app.assistant.provider;
    if !provider.needs_key() {
        cli_status(app, ui, provider);
        ui.separator();
        prompt_section(app, ui);
        return;
    }
    ui.horizontal(|ui| {
        ui.label("API key");
        ui.add(
            egui::TextEdit::singleline(&mut app.assistant.key_input)
                .password(true)
                .desired_width(220.0)
                .hint_text(provider.env_var().unwrap_or_default()),
        );
        if ui
            .add_enabled(
                !app.assistant.key_input.trim().is_empty(),
                egui::Button::new("Save"),
            )
            .on_hover_text("Store this key for next time, outside every project")
            .clicked()
        {
            let key = otd_ai::Key::new(app.assistant.key_input.clone());
            app.assistant.keys.set(provider, key);
            match app.assistant.keys.save() {
                Ok(path) => {
                    app.assistant.key_input.clear();
                    app.assistant.status = format!("Key saved to {}", path.display());
                    app.assistant.error = None;
                }
                Err(e) => app.assistant.error = Some(e),
            }
        }
    });
    ui.horizontal(|ui| {
        match app.assistant.keys.get(provider) {
            Some(key) => ui.label(
                RichText::new(format!("stored: {}", key.hint()))
                    .weak()
                    .small(),
            ),
            None => ui.label(
                RichText::new(format!(
                    "no key — paste one, or set {} in the environment",
                    provider.env_var().unwrap_or_default()
                ))
                .weak()
                .small(),
            ),
        };
        ui.hyperlink_to(RichText::new("get one").small(), provider.console_url());
    });

    ui.separator();
    prompt_section(app, ui);
}
pub(crate) fn cli_status(app: &mut OtdApp, ui: &mut egui::Ui, provider: Provider) {
    if !app.assistant.cli_version.contains_key(&provider) {
        app.assistant.recheck_cli(provider);
    }

    ui.horizontal(|ui| {
        ui.label("Signed in via");
        match app.assistant.cli_version.get(&provider) {
            Some(Ok(version)) => {
                ui.label(RichText::new(version).monospace().small());
            }
            Some(Err(e)) => {
                ui.label(
                    RichText::new(e)
                        .small()
                        .color(Color32::from_rgb(235, 170, 90)),
                );
            }
            None => {
                ui.label(RichText::new("checking…").weak().small());
            }
        }
        if ui
            .small_button("Re-check")
            .on_hover_text("Look for the CLI again — after installing it, say")
            .clicked()
        {
            app.assistant.recheck_cli(provider);
        }
    });
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(
                "Uses this machine's own login, so it costs subscription quota rather than \
                 API credit. Nothing is billed per token and no key is stored.",
            )
            .weak()
            .small(),
        );
    });
    ui.hyperlink_to(
        RichText::new("install and sign in").small(),
        provider.console_url(),
    );
}
pub(crate) fn prompt_section(app: &mut OtdApp, ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!(
            "Builds into {} — the network you are looking at.",
            app.graph.path(app.canvas.current)
        ))
        .weak()
        .small(),
    );
    ui.add(
        egui::TextEdit::multiline(&mut app.assistant.prompt)
            .desired_rows(3)
            .desired_width(f32::INFINITY)
            .hint_text(match app.assistant.image.is_some() {
                true => "What to change about the reference — or nothing, to just build it…",
                false => "Describe what you want to see…",
            }),
    );

    ui.horizontal(|ui| {
        ui.label(RichText::new("Reference").weak().small());
        attachment_row(app, ui);
        if app.assistant.image.is_none() {
            ui.label(
                RichText::new("a still to work back from — the look is rebuilt as operators")
                    .weak()
                    .small(),
            );
        }
    });

    ui.horizontal_wrapped(|ui| {
        for suggestion in suggestions() {
            if ui.small_button(suggestion).clicked() {
                app.assistant.prompt = suggestion.to_string();
            }
        }
    });

    ui.horizontal(|ui| {
        let ready = !app.assistant.busy() && app.assistant.has_request();
        if ui
            .add_enabled(ready, egui::Button::new("Build it"))
            .clicked()
        {
            send(app);
        }
        if app.assistant.busy() {
            ui.spinner();
            ui.label(RichText::new("thinking…").weak());
        }
        if !app.assistant.status.is_empty() {
            ui.label(RichText::new(&app.assistant.status).weak().small());
        }
    });

    // ---- what happened
    if let Some(error) = &app.assistant.error {
        ui.colored_label(Color32::from_rgb(235, 120, 120), error);
    }
    if let Some(notes) = &app.assistant.last {
        ui.separator();
        ui.label(notes);
    }
    // Said out loud, above the notes and never folded away. This is the one
    // failure that looks like a success: the nodes are all there, the status
    // line counts them, and the picture is black from the red node down.
    if !app.assistant.broken.is_empty() {
        ui.separator();
        for (node, error) in &app.assistant.broken {
            ui.colored_label(
                Color32::from_rgb(235, 120, 120),
                format!("{node}: shader will not compile — everything after it is black"),
            );
            ui.label(RichText::new(error).small().monospace().weak());
        }
    }
    if !app.assistant.warnings.is_empty() {
        ui.collapsing(
            format!("{} thing(s) skipped", app.assistant.warnings.len()),
            |ui| {
                for warning in &app.assistant.warnings {
                    ui.label(RichText::new(warning).small().monospace());
                }
            },
        );
    }
    if app.assistant.last.is_some() {
        ui.label(
            RichText::new("Not what you wanted? Cmd/Ctrl+Z undoes the whole thing.")
                .weak()
                .small(),
        );
    }
}
