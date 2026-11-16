//! Editor color palettes — network chrome and egui chrome, not project media.
use egui::Color32;
use otd_core::Family;


use super::helpers::family_index;

/// Colors and drawing style for the network editor.
#[derive(Clone, Copy, Debug)]
pub struct NetworkTheme {
    pub canvas_bg: Color32,
    pub grid: Color32,
    pub grid_dots: bool,
    pub node_corner: f32,
    pub node_fill: Color32,
    pub node_fill_sel: Color32,
    pub node_body: Color32,
    pub node_footer: Color32,
    pub node_name_bg: Color32,
    pub node_rail: Color32,
    pub node_text: Color32,
    pub node_icon_muted: Color32,
    pub sel_primary: Color32,
    pub sel_secondary: Color32,
    pub backdrop_fill: Color32,
    pub backdrop_border: Color32,
    pub backdrop_label: Color32,
    pub families: [[u8; 3]; 6],
    pub channels: [[u8; 3]; 6],
}

impl NetworkTheme {
    pub fn family_color(self, f: Family) -> Color32 {
        let c = self.families[family_index(f)];
        Color32::from_rgb(c[0], c[1], c[2])
    }

    pub fn channel_color(self, index: usize) -> Color32 {
        let c = self.channels[index % self.channels.len()];
        Color32::from_rgb(c[0], c[1], c[2])
    }
}
