use egui::{Align2, Color32, FontId, Rect, Vec2};

use crate::app::OtdApp;

pub(crate) fn empty_hint(app: &mut OtdApp, ui: &mut egui::Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    let centre = rect.center();
    painter.text(
        centre - Vec2::new(0.0, 74.0),
        Align2::CENTER_CENTER,
        "Drop a video, image or audio file here",
        FontId::proportional(20.0),
        Color32::from_rgb(190, 196, 210),
    );
    painter.text(
        centre - Vec2::new(0.0, 46.0),
        Align2::CENTER_CENTER,
        "it starts playing, and everything downstream is yours to build",
        FontId::proportional(13.0),
        Color32::from_gray(125),
    );

    let button = Rect::from_center_size(centre, Vec2::new(190.0, 30.0));
    if ui.put(button, egui::Button::new("Import media…")).clicked() {
        crate::media::import_dialog(app);
    }
    let webcam = Rect::from_center_size(centre + Vec2::new(0.0, 38.0), Vec2::new(190.0, 30.0));
    if ui.put(webcam, egui::Button::new("Use webcam")).clicked() {
        crate::media::add_webcam(app);
    }

    painter.text(
        centre + Vec2::new(0.0, 84.0),
        Align2::CENTER_CENTER,
        "or press Tab or double-click here for any of the 126 operators",
        FontId::proportional(13.0),
        Color32::from_gray(110),
    );
}
