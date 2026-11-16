
use egui::RichText;
use otd_ai::recipes::{self};

use crate::app::OtdApp;

use super::drop::IMAGE_EXTENSIONS;

pub(crate) fn attachment_row(app: &mut OtdApp, ui: &mut egui::Ui) {
    let texture = app.assistant.thumbnail(ui.ctx());
    let Some(image) = &app.assistant.image else {
        if ui
            .small_button("📎")
            .on_hover_text("Attach a reference image to work back from — or drop one on the bar")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", IMAGE_EXTENSIONS)
                .pick_file()
            {
                app.assistant.attach(&path);
            }
        }
        return;
    };

    let name = image.name();
    let detail = image.detail();
    if let Some(texture) = texture {
        ui.add(
            egui::Image::new(&texture)
                .max_height(20.0)
                .corner_radius(3.0),
        )
        .on_hover_text(format!("{name} — {detail}"));
    }
    ui.label(RichText::new(short_name(&name)).small().weak())
        .on_hover_text(format!(
            "{name} — {detail}\nThe patch is built to match this."
        ));
    if ui
        .small_button("×")
        .on_hover_text("Remove the reference image")
        .clicked()
    {
        app.assistant.detach();
    }
}
pub(crate) fn short_name(name: &str) -> String {
    if name.chars().count() <= 18 {
        return name.to_string();
    }
    let head: String = name.chars().take(10).collect();
    let tail: String = name
        .chars()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}…{tail}")
}
pub(crate) fn model_label(model: &str) -> &str {
    match model.trim().is_empty() {
        true => "(CLI default)",
        false => model,
    }
}
pub(crate) fn suggestions() -> Vec<&'static str> {
    recipes::groups()
        .into_iter()
        .filter_map(|g| recipes::all().iter().find(|r| r.group == g))
        .map(|r| r.prompt.as_str())
        .collect()
}
