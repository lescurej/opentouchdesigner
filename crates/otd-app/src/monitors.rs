//! Monitor layout for Window COMP placement (display index 0 is the first screen).

use egui::{Context, Pos2, Vec2};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MonitorLayout {
    pub origin: Pos2,
    pub size: Vec2,
}

impl MonitorLayout {
    pub fn fallback() -> Self {
        Self {
            origin: Pos2::ZERO,
            size: Vec2::new(1920.0, 1080.0),
        }
    }
}

pub fn snapshot(ctx: &Context) -> Vec<MonitorLayout> {
    #[cfg(target_os = "macos")]
    {
        let screens = macos_screens();
        if !screens.is_empty() {
            return screens;
        }
    }
    let size = ctx
        .input(|i| i.viewport().monitor_size)
        .unwrap_or_else(|| Vec2::new(1920.0, 1080.0));
    vec![MonitorLayout {
        origin: Pos2::ZERO,
        size,
    }]
}

pub fn layout_at(monitors: &[MonitorLayout], index: usize) -> MonitorLayout {
    monitors
        .get(index)
        .or_else(|| monitors.first())
        .copied()
        .unwrap_or_else(MonitorLayout::fallback)
}

#[cfg(target_os = "macos")]
fn macos_screens() -> Vec<MonitorLayout> {
    use objc2_app_kit::NSScreen;
    use objc2_foundation::MainThreadMarker;
    let Some(mtm) = MainThreadMarker::new() else {
        return Vec::new();
    };
    let screens = NSScreen::screens(mtm);
    let frames: Vec<_> = screens.iter().map(|screen| screen.frame()).collect();
    let top = frames
        .iter()
        .map(|frame| frame.origin.y + frame.size.height)
        .fold(0.0, f64::max);
    frames
        .into_iter()
        .map(|frame| MonitorLayout {
            origin: Pos2::new(
                frame.origin.x as f32,
                (top - (frame.origin.y + frame.size.height)) as f32,
            ),
            size: Vec2::new(frame.size.width as f32, frame.size.height as f32),
        })
        .collect()
}
