use egui::Ui;
use otd_core::Family;
use otd_core::NodeId;

use crate::app::OtdApp;

use super::widgets::{td_param_label, td_toggle};

pub fn panel_common_tab(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    ui.spacing_mut().item_spacing.y = 4.0;
    let mut flags: Vec<(&str, bool, crate::canvas::input::SelectionFlag)> = vec![
        (
            "Bypass",
            app.graph.node(id).flags.bypass,
            crate::canvas::input::SelectionFlag::Bypass,
        ),
        (
            "Viewer",
            app.graph.node(id).flags.viewer,
            crate::canvas::input::SelectionFlag::Viewer,
        ),
        (
            "Clone Immune",
            app.graph.node(id).flags.clone_immune,
            crate::canvas::input::SelectionFlag::CloneImmune,
        ),
        (
            "Viewer Active",
            app.graph.node(id).flags.viewer_active,
            crate::canvas::input::SelectionFlag::ViewerActive,
        ),
        (
            "Lock",
            app.graph.node(id).flags.lock,
            crate::canvas::input::SelectionFlag::Lock,
        ),
    ];
    if app.graph.node(id).family != Family::Comp && app.graph.node(id).family != Family::Mat {
        flags.insert(
            3,
            (
                "Display",
                app.graph.node(id).flags.bg_display,
                crate::canvas::input::SelectionFlag::Display,
            ),
        );
    }
    for (label, on, flag) in flags {
        let mut v = on;
        ui.horizontal(|ui| {
            td_param_label(ui, label);
            if td_toggle(ui, &mut v) && v != on {
                    crate::canvas::input::toggle_flag_at(app, flag, id);
            }
        });
    }
    if app.graph.node(id).family == Family::Comp {
        let mut perform = app.perform_ui() == Some(id);
        ui.horizontal(|ui| {
            td_param_label(ui, "Perform UI");
            if td_toggle(ui, &mut perform) && perform != (app.perform_ui() == Some(id)) {
                app.toggle_perform_ui(id);
            }
        });
    }
}

pub fn panel_extensions_tab(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    crate::params::component_links(app, ui, id);
    crate::params::custom_param_editor(app, ui, id);
}