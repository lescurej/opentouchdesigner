//! Editor color palettes — network chrome and egui chrome, not project media.
use egui::Color32;
use otd_core::Family;

use crate::settings::UiPalette;

use super::theme::NetworkTheme;

pub(crate) fn family_index(f: Family) -> usize {
    match f {
        Family::Top => 0,
        Family::Chop => 1,
        Family::Sop => 2,
        Family::Dat => 3,
        Family::Mat => 4,
        Family::Comp => 5,
    }
}

pub(crate) fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn lerp_toward_white(c: Color32, white_blend: f32) -> Color32 {
    let b = white_blend.clamp(0.0, 1.0);
    Color32::from_rgb(
        (f32::from(c.r()) * (1.0 - b) + 255.0 * b).round() as u8,
        (f32::from(c.g()) * (1.0 - b) + 255.0 * b).round() as u8,
        (f32::from(c.b()) * (1.0 - b) + 255.0 * b).round() as u8,
    )
}

fn light_network_text(dark: NetworkTheme, palette: UiPalette) -> Color32 {
    match palette {
        UiPalette::Paper => dark.node_text,
        UiPalette::Terminal => rgb(22, 72, 38),
        _ => {
            if dark.node_text.r() > 175 && dark.node_text.g() > 175 && dark.node_text.b() > 175 {
                let tint = dark.canvas_bg;
                rgb(
                    (tint.r() / 2).clamp(32, 72),
                    (tint.g() / 2).clamp(32, 72),
                    (tint.b() / 2).clamp(36, 80),
                )
            } else {
                rgb(36, 38, 44)
            }
        }
    }
}

pub(crate) fn lighten_network_theme(dark: NetworkTheme, palette: UiPalette) -> NetworkTheme {
    let icon = match palette {
        UiPalette::Terminal => rgb(56, 108, 72),
        _ => lerp_toward_white(dark.node_icon_muted, 0.45),
    };
    NetworkTheme {
        canvas_bg: lerp_toward_white(dark.canvas_bg, 0.86),
        grid: lerp_toward_white(dark.grid, 0.68),
        grid_dots: dark.grid_dots,
        node_corner: dark.node_corner,
        node_fill: lerp_toward_white(dark.node_fill, 0.9),
        node_fill_sel: lerp_toward_white(dark.node_fill_sel, 0.92),
        node_body: lerp_toward_white(dark.node_body, 0.88),
        node_footer: lerp_toward_white(dark.node_footer, 0.82),
        node_name_bg: lerp_toward_white(dark.node_name_bg, 0.9),
        node_rail: lerp_toward_white(dark.node_rail, 0.62),
        node_text: light_network_text(dark, palette),
        node_icon_muted: icon,
        sel_primary: dark.sel_primary,
        sel_secondary: dark.sel_secondary,
        backdrop_fill: lerp_toward_white(dark.backdrop_fill, 0.9),
        backdrop_border: lerp_toward_white(dark.backdrop_border, 0.55),
        backdrop_label: rgb(56, 58, 64),
        families: dark.families,
        channels: dark.channels,
    }
}
