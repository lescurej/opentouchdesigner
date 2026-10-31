use egui::{Pos2, Rect, Vec2};

pub const NODE_W: f32 = 176.0;
pub const BODY_H: f32 = 99.0;
pub const FOOTER_H: f32 = 18.0;
pub const NODE_H: f32 = BODY_H + FOOTER_H;
pub const CALLBACK_FOLD_H: f32 = 16.0;

#[derive(Clone, Copy, Debug)]
pub struct CanvasView {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Default for CanvasView {
    fn default() -> Self {
        CanvasView {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl CanvasView {
    pub(crate) fn to_screen(self, origin: Pos2, world: Vec2) -> Pos2 {
        origin + (world + self.pan) * self.zoom
    }

    pub fn to_world(self, origin: Pos2, screen: Pos2) -> Vec2 {
        (screen - origin) / self.zoom - self.pan
    }

    pub(crate) fn fit_world_bounds(
        origin: Pos2,
        canvas: Rect,
        min: Vec2,
        max: Vec2,
    ) -> Self {
        if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
            return Self::default();
        }
        let size = max - min;
        if size.x < 1.0 || size.y < 1.0 {
            return Self::default();
        }
        const SCREEN_MARGIN: f32 = 40.0;
        const WORLD_PAD: f32 = 24.0;
        let content = size + Vec2::splat(WORLD_PAD * 2.0);
        let avail = canvas.size() - Vec2::splat(SCREEN_MARGIN * 2.0);
        let zoom = (avail.x / content.x)
            .min(avail.y / content.y)
            .clamp(0.15, 3.0);
        let center = (min + max) * 0.5;
        let pan = (canvas.center() - origin) / zoom - center;
        Self { pan, zoom }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum DragState {
    Node { id: otd_core::NodeId, grab: Vec2 },
    Wire {
        from: otd_core::NodeId,
        output: usize,
    },
    WireTo {
        dst: otd_core::NodeId,
        input: usize,
        had_wire: bool,
    },
    Marquee {
        start: Pos2,
        additive: bool,
    },
}
