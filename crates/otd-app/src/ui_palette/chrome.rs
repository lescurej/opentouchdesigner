//! Editor color palettes — network chrome and egui chrome, not project media.
use egui::{Context, Theme, ThemePreference, Visuals};

use crate::settings::{
    UiPalette, UiTheme,
};

use super::helpers::rgb;
use super::network::network_for_appearance;

/// Whether chrome and palette tints should use dark [`Visuals`].
pub fn effective_dark(
    preference: UiTheme,
    palette: UiPalette,
    system_theme: Option<Theme>,
    ctx_resolved: Theme,
) -> bool {
    if palette == UiPalette::Paper {
        return false;
    }
    match preference {
        UiTheme::Light => false,
        UiTheme::Dark => true,
        UiTheme::System => match system_theme {
            Some(Theme::Dark) => true,
            Some(Theme::Light) => false,
            None => matches!(ctx_resolved, Theme::Dark),
        },
    }
}

pub fn apply(ctx: &Context, palette: UiPalette, theme: UiTheme, system_theme: Option<Theme>) {
    if palette == UiPalette::Paper {
        ctx.set_theme(ThemePreference::Light);
    } else {
        ctx.set_theme(theme.to_egui());
    }
    let dark = effective_dark(theme, palette, system_theme, ctx.theme());
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    tint_visuals(palette, &mut visuals);
    ctx.set_visuals(visuals);
}

fn tint_visuals(palette: UiPalette, v: &mut Visuals) {
    let net = network_for_appearance(palette, v.dark_mode);
    v.panel_fill = net
        .canvas_bg
        .gamma_multiply(if v.dark_mode { 1.05 } else { 1.0 });
    v.window_fill = if v.dark_mode {
        net.node_fill.gamma_multiply(1.1)
    } else {
        net.node_fill
    };
    v.extreme_bg_color = net.node_name_bg;
    v.faint_bg_color = net.node_body;
    let sel_bg = if v.dark_mode { 0.35 } else { 0.22 };
    v.selection.bg_fill = net.sel_secondary.gamma_multiply(sel_bg);
    v.selection.stroke.color = net.sel_primary;

    if !v.dark_mode {
        v.override_text_color = None;
        v.widgets.noninteractive.fg_stroke.color = net.node_text;
        v.widgets.inactive.fg_stroke.color = net.node_text;
        v.widgets.hovered.fg_stroke.color = net.node_text;
        v.widgets.active.fg_stroke.color = net.node_text;
    }

    match palette {
        UiPalette::Classic => {}
        UiPalette::Ocean => {
            v.hyperlink_color = rgb(120, 190, 255);
            if v.dark_mode {
                v.widgets.active.bg_fill = rgb(40, 90, 140);
            } else {
                v.widgets.active.bg_fill = rgb(180, 210, 240);
            }
        }
        UiPalette::Paper => {
            v.widgets.noninteractive.fg_stroke.color = rgb(50, 48, 44);
            v.widgets.inactive.bg_fill = rgb(240, 236, 228);
        }
        UiPalette::Terminal => {
            if v.dark_mode {
                v.override_text_color = Some(rgb(170, 240, 180));
                v.widgets.active.bg_fill = rgb(40, 100, 50);
                v.hyperlink_color = rgb(120, 220, 140);
            } else {
                v.hyperlink_color = rgb(32, 130, 58);
                v.widgets.active.bg_fill = rgb(190, 230, 200);
            }
        }
        UiPalette::Vivid => {
            if v.dark_mode {
                v.widgets.active.bg_fill = rgb(120, 60, 180);
            } else {
                v.widgets.active.bg_fill = rgb(210, 180, 240);
            }
            v.hyperlink_color = rgb(220, 140, 255);
        }
        UiPalette::Ember => {
            if v.dark_mode {
                v.widgets.active.bg_fill = rgb(120, 60, 40);
            } else {
                v.widgets.active.bg_fill = rgb(240, 200, 175);
            }
            v.hyperlink_color = rgb(255, 170, 100);
        }
        UiPalette::Synthwave => {
            if v.dark_mode {
                v.widgets.active.bg_fill = rgb(100, 40, 120);
            } else {
                v.widgets.active.bg_fill = rgb(220, 190, 235);
            }
            v.hyperlink_color = if v.dark_mode {
                rgb(0, 220, 255)
            } else {
                rgb(0, 140, 190)
            };
        }
        UiPalette::Midnight => {
            if v.dark_mode {
                v.widgets.active.bg_fill = rgb(50, 60, 100);
            } else {
                v.widgets.active.bg_fill = rgb(190, 200, 230);
            }
            v.hyperlink_color = rgb(180, 200, 255);
        }
        UiPalette::Aurora => {
            if v.dark_mode {
                v.widgets.active.bg_fill = rgb(40, 100, 80);
            } else {
                v.widgets.active.bg_fill = rgb(180, 230, 210);
            }
            v.hyperlink_color = rgb(140, 255, 210);
        }
        UiPalette::Ink => {
            if v.dark_mode {
                v.widgets.active.bg_fill = rgb(70, 68, 90);
            } else {
                v.widgets.active.bg_fill = rgb(210, 208, 220);
            }
            v.hyperlink_color = rgb(220, 90, 75);
        }
    }

    let rounding = match palette {
        UiPalette::Classic | UiPalette::Terminal | UiPalette::Ink => 2.0,
        UiPalette::Ocean => 5.0,
        UiPalette::Paper => 4.0,
        UiPalette::Vivid | UiPalette::Aurora => 8.0,
        UiPalette::Ember | UiPalette::Synthwave => 4.0,
        UiPalette::Midnight => 3.0,
    };
    v.widgets.noninteractive.corner_radius = rounding.into();
    v.widgets.inactive.corner_radius = rounding.into();
    v.widgets.hovered.corner_radius = rounding.into();
    v.widgets.active.corner_radius = rounding.into();
}
