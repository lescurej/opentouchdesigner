//! Editor color palettes — network chrome and egui chrome, not project media.
use egui::Color32;

use crate::settings::{
    AppSettings, NetworkGridStyle, UiPalette, clamp_network_canvas_bg_gamma,
    clamp_network_grid_opacity,
};

use super::helpers::lighten_network_theme;
use super::theme::NetworkTheme;


/// Network colors for the graph and chrome tint: dark palettes lighten when the UI is light.
pub fn network_for_appearance(palette: UiPalette, chrome_dark: bool) -> NetworkTheme {
    if chrome_dark {
        return network(palette);
    }
    if palette == UiPalette::Paper {
        network(UiPalette::Paper)
    } else {
        lighten_network_theme(network(palette), palette)
    }
}

impl UiPalette {
    pub fn label(self) -> &'static str {
        match self {
            UiPalette::Classic => "Classic",
            UiPalette::Ocean => "Ocean",
            UiPalette::Paper => "Paper",
            UiPalette::Terminal => "Terminal",
            UiPalette::Vivid => "Vivid",
            UiPalette::Ember => "Ember",
            UiPalette::Synthwave => "Synthwave",
            UiPalette::Midnight => "Midnight",
            UiPalette::Aurora => "Aurora",
            UiPalette::Ink => "Ink",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            UiPalette::Classic => "Default dark network — sharp nodes, neutral grid",
            UiPalette::Ocean => "Cool blues, soft grid dots, rounded nodes",
            UiPalette::Paper => "Light studio look — warm greys, crisp wires",
            UiPalette::Terminal => "Phosphor green on black, monospace feel",
            UiPalette::Vivid => "Deep violet canvas, neon operator colors",
            UiPalette::Ember => "Volcanic warmth — copper rails, ember operator hues",
            UiPalette::Synthwave => "Retro dusk — magenta sky, cyan wires, dot grid",
            UiPalette::Midnight => "Deep navy canvas, gold selection, jewel families",
            UiPalette::Aurora => "Polar night — teal glow, violet haze, soft corners",
            UiPalette::Ink => "Sumi-e dark — indigo wash, vermillion accent",
        }
    }
}

pub fn apply_network_appearance(theme: &mut NetworkTheme, settings: &AppSettings) {
    let opacity = clamp_network_grid_opacity(settings.network_grid_opacity);
    if opacity <= 0.0 {
        theme.grid = Color32::TRANSPARENT;
    } else if opacity < 1.0 {
        let c = theme.grid;
        theme.grid = Color32::from_rgba_unmultiplied(
            c.r(),
            c.g(),
            c.b(),
            ((c.a() as f32) * opacity).round().clamp(0.0, 255.0) as u8,
        );
    }
    match settings.network_grid_style {
        NetworkGridStyle::Palette => {}
        NetworkGridStyle::Lines => theme.grid_dots = false,
        NetworkGridStyle::Dots => theme.grid_dots = true,
    }
    let gamma = clamp_network_canvas_bg_gamma(settings.network_canvas_bg_gamma);
    if (gamma - 1.0).abs() > f32::EPSILON {
        theme.canvas_bg = theme.canvas_bg.gamma_multiply(gamma);
    }
}
use super::palettes::network_palettes;

pub fn network(palette: UiPalette) -> NetworkTheme {
    network_palettes(palette)
}
