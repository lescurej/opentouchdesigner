use std::cell::Cell;

use egui::Color32;
use otd_core::Family;

use crate::ui_palette::{NetworkTheme, network};

thread_local! {
    static ACTIVE: Cell<NetworkTheme> = Cell::new(network(crate::settings::UiPalette::Classic));
}

pub(crate) fn set_active(theme: NetworkTheme) {
    ACTIVE.set(theme);
}

fn active() -> NetworkTheme {
    ACTIVE.with(|t| t.get())
}

pub fn channel_color(index: usize) -> Color32 {
    active().channel_color(index)
}

pub(crate) fn family_color(f: Family) -> Color32 {
    active().family_color(f)
}

pub(crate) fn canvas_bg() -> Color32 {
    active().canvas_bg
}

pub(crate) fn grid_color() -> Color32 {
    active().grid
}

pub(crate) fn grid_dots() -> bool {
    active().grid_dots
}

pub(crate) fn node_fill(selected: bool) -> Color32 {
    if selected {
        active().node_fill_sel
    } else {
        active().node_fill
    }
}

pub(crate) fn selection_color(primary: bool) -> Color32 {
    if primary {
        active().sel_primary
    } else {
        active().sel_secondary
    }
}

pub(crate) fn node_corner() -> f32 {
    active().node_corner
}
