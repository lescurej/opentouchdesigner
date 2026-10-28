use egui::Color32;
use otd_core::Family;

pub fn channel_color(index: usize) -> Color32 {
    const PALETTE: [[u8; 3]; 6] = [
        [126, 190, 126],
        [126, 170, 220],
        [220, 180, 110],
        [200, 130, 200],
        [220, 130, 130],
        [140, 210, 200],
    ];
    let c = PALETTE[index % PALETTE.len()];
    Color32::from_rgb(c[0], c[1], c[2])
}

pub(crate) fn family_color(f: Family) -> Color32 {
    let c = f.color();
    Color32::from_rgb(c[0], c[1], c[2])
}
