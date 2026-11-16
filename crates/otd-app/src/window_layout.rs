use egui::{Pos2, Vec2, ViewportBuilder, WindowLevel};
use otd_core::{EvalContext, Graph, NodeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JustifyArea {
    Primary,
    SpecifyDisplay,
    AllDisplays,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JustifyAxis {
    Start,
    Center,
    End,
    Mouse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpeningSize {
    Automatic,
    Fill,
    Custom,
    Exclusive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureStretch {
    Fit,
    Fill,
    Stretch,
    Native,
}

pub struct WindowCompConfig {
    pub title: String,
    pub borders: bool,
    pub always_on_top: bool,
    pub close_on_escape: bool,
    pub display_index: usize,
    pub justify_area: JustifyArea,
    pub justify_h: JustifyAxis,
    pub justify_v: JustifyAxis,
    pub offset: Vec2,
    pub opening_size: OpeningSize,
    pub custom_w: f32,
    pub custom_h: f32,
    pub stretch: TextureStretch,
    pub cursor_visible: bool,
    pub allow_minimize: bool,
}

impl Default for WindowCompConfig {
    fn default() -> Self {
        Self {
            title: "Window".into(),
            borders: true,
            always_on_top: false,
            close_on_escape: true,
            display_index: 0,
            justify_area: JustifyArea::Primary,
            justify_h: JustifyAxis::Center,
            justify_v: JustifyAxis::Center,
            offset: Vec2::ZERO,
            opening_size: OpeningSize::Custom,
            custom_w: 1280.0,
            custom_h: 720.0,
            stretch: TextureStretch::Fit,
            cursor_visible: true,
            allow_minimize: true,
        }
    }
}

pub fn window_comp_config(graph: &Graph, id: NodeId) -> WindowCompConfig {
    let node = graph.node(id);
    let ctx = EvalContext::default();
    let s = |key: &str| {
        node.param(key)
            .map(|p| p.eval(&ctx).as_str())
            .unwrap_or_default()
    };
    let f = |key: &str, d: f64| node.param(key).map(|p| p.eval(&ctx).as_f64()).unwrap_or(d);
    let b = |key: &str, d: bool| node.param(key).map(|p| p.eval(&ctx).as_bool()).unwrap_or(d);

    let justify_area = match s("justifyoffsetto").as_str() {
        "specifydisplay" => JustifyArea::SpecifyDisplay,
        "alldisplays" => JustifyArea::AllDisplays,
        _ => JustifyArea::Primary,
    };
    let justify_h = match s("justifyh").as_str() {
        "left" => JustifyAxis::Start,
        "right" => JustifyAxis::End,
        "mouse" => JustifyAxis::Mouse,
        _ => JustifyAxis::Center,
    };
    let justify_v = match s("justifyv").as_str() {
        "top" => JustifyAxis::Start,
        "bottom" => JustifyAxis::End,
        "mouse" => JustifyAxis::Mouse,
        _ => JustifyAxis::Center,
    };
    let opening_size = match s("openingsize").as_str() {
        "automatic" => OpeningSize::Automatic,
        "fill" => OpeningSize::Fill,
        "exclusive" => OpeningSize::Exclusive,
        _ => OpeningSize::Custom,
    };
    let stretch = match s("stretch").as_str() {
        "fill" => TextureStretch::Fill,
        "stretch" => TextureStretch::Stretch,
        "native" | "nativeres" => TextureStretch::Native,
        _ => TextureStretch::Fit,
    };

    let display_index = f("display", 0.0).max(0.0) as usize;
    let title = s("title");
    WindowCompConfig {
        title: if title.trim().is_empty() {
            "OpenTouchDesigner — Window".into()
        } else {
            title
        },
        borders: b("borders", true),
        always_on_top: b("alwaysontop", false),
        close_on_escape: b("closeescape", true),
        display_index,
        justify_area,
        justify_h,
        justify_v,
        offset: Vec2::new(f("winoffsetx", 0.0) as f32, f("winoffsety", 0.0) as f32),
        opening_size,
        custom_w: f("winw", 1280.0).clamp(64.0, 16384.0) as f32,
        custom_h: f("winh", 720.0).clamp(64.0, 16384.0) as f32,
        stretch,
        cursor_visible: b("cursorvisible", true),
        allow_minimize: b("allowminimize", true),
    }
}

pub fn display_index_for_config(cfg: &WindowCompConfig) -> usize {
    match cfg.justify_area {
        JustifyArea::SpecifyDisplay => cfg.display_index,
        JustifyArea::AllDisplays | JustifyArea::Primary => 0,
    }
}

pub fn inner_size_for_texture(cfg: &WindowCompConfig, tex: Option<[u32; 2]>) -> Vec2 {
    match cfg.opening_size {
        OpeningSize::Automatic => tex
            .map(|[w, h]| Vec2::new(w.max(1) as f32, h.max(1) as f32))
            .unwrap_or_else(|| Vec2::new(cfg.custom_w, cfg.custom_h)),
        OpeningSize::Custom => Vec2::new(cfg.custom_w, cfg.custom_h),
        OpeningSize::Fill | OpeningSize::Exclusive => Vec2::new(1280.0, 720.0),
    }
}

pub fn build_viewport_builder(
    cfg: &WindowCompConfig,
    tex_size: Option<[u32; 2]>,
    anchor: Option<Pos2>,
    monitor: Option<crate::monitors::MonitorLayout>,
) -> ViewportBuilder {
    let mut builder = ViewportBuilder::default().with_title(cfg.title.clone());

    let fullscreen_monitor = matches!(cfg.opening_size, OpeningSize::Fill | OpeningSize::Exclusive);
    if fullscreen_monitor {
        let idx = display_index_for_config(cfg);
        builder = builder
            .with_monitor(idx)
            .with_decorations(false)
            .with_resizable(false);
    } else {
        let size = inner_size_for_texture(cfg, tex_size);
        builder = builder
            .with_inner_size([size.x, size.y])
            .with_decorations(cfg.borders)
            .with_resizable(true);
        if !cfg.allow_minimize {
            builder = builder.with_minimize_button(false);
        }
        if let Some(pos) = window_outer_position(cfg, size, anchor, monitor) {
            builder = builder.with_position(pos);
        }
    }

    if cfg.always_on_top {
        builder = builder.with_window_level(WindowLevel::AlwaysOnTop);
    }

    builder
}

fn window_outer_position(
    cfg: &WindowCompConfig,
    inner: Vec2,
    anchor: Option<Pos2>,
    monitor: Option<crate::monitors::MonitorLayout>,
) -> Option<Pos2> {
    let layout = monitor.unwrap_or_else(crate::monitors::MonitorLayout::fallback);
    let area = layout.size;
    let origin = layout.origin;
    let frame = estimate_frame(cfg.borders);
    let outer = inner + frame;

    let ax = match cfg.justify_h {
        JustifyAxis::Start => origin.x,
        JustifyAxis::End => origin.x + (area.x - outer.x).max(0.0),
        JustifyAxis::Mouse => anchor
            .map(|p| p.x - outer.x * 0.5)
            .unwrap_or(origin.x + area.x * 0.5),
        JustifyAxis::Center => origin.x + (area.x - outer.x) * 0.5,
    };
    let ay = match cfg.justify_v {
        JustifyAxis::Start => origin.y,
        JustifyAxis::End => origin.y + (area.y - outer.y).max(0.0),
        JustifyAxis::Mouse => anchor
            .map(|p| p.y - outer.y * 0.5)
            .unwrap_or(origin.y + area.y * 0.5),
        JustifyAxis::Center => origin.y + (area.y - outer.y) * 0.5,
    };
    Some(Pos2::new(ax + cfg.offset.x, ay + cfg.offset.y))
}

fn estimate_frame(borders: bool) -> Vec2 {
    if borders {
        Vec2::new(0.0, 28.0)
    } else {
        Vec2::ZERO
    }
}
