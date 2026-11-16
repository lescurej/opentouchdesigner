//! Panel components — the widgets a show is driven from.
//!
//! TouchDesigner's panel COMPs are a control surface built out of the same
//! node graph as everything else, and that is the part worth copying: a button
//! is a node, its state is a parameter, and a parameter is already something
//! the rest of the network can export from, bind to and animate. So there is
//! no new value system here and no widget-to-operator plumbing — a Slider COMP
//! is a node whose `value` parameter happens to have a person moving it.
//!
//! Three consequences fall out of that choice, all of them good:
//!
//!  * A panel's state is **in the project file**, because parameters are.
//!    Reopening a show restores the fader positions.
//!  * Undo works on it, because parameter edits are what undo is made of.
//!  * A widget can be driven by the network as readily as it drives the
//!    network — exporting a CHOP to a slider's `value` moves the slider.
//!
//! Layout is in **fractions of the output**, not pixels. A panel laid out
//! against a 1280×720 viewer has to land in the same place on a 4K projector,
//! and the alternative — pixels plus a design resolution — is a second number
//! to keep in sync for no benefit.
//!
//! ## Elementary bricks (TouchDesigner-shaped)
//!
//! * A **containerCOMP** is a panel surface: it lays out **direct panel-child
//!   COMPs** and draws them in that rectangle. Nested containers recurse the
//!   same rule; nothing flattens the whole subtree into one list.
//! * A **button/slider/field/text COMP** is still a COMP: its pixels come from
//!   its internal cooked TOP chain (`view1`, interior `textCOMP`, etc.).
//! * Any **TOP** (or non-panel COMP) that is not part of that panel tree is
//!   shown on a panel only through **opviewerCOMP** (`op` → TOP or COMP).
//! * **View** on a container opens a floating window at **View Width/Height**
//!   (`vieww`/`viewh`, default 400×300) with the same panel renderer as Perform.
//! * **Perform** is fullscreen on the project perform-ui container.
//! * **windowCOMP** is unrelated: it only blits a **TOP** to a floating or
//!   external OS window (`winop`), never panel layout.

mod defs;

pub use defs::{
    defs, register, BUTTON, CONTAINER, CONTAINER_DEF, FIELD, GLSL_PANEL, LIST, OP_VIEWER,
    PARAMETER, SELECT, SLIDER, TABLE, TEXT, WIDGET, is_panel, is_panel_type, panel_display_visible,
};

use otd_core::{EvalContext, Graph, NodeId};

/// One widget, resolved out of the graph and ready to draw.
///
/// The editor gets this rather than a `NodeId` and a pile of parameter
/// lookups, so the "what is on the panel" question is answered here — in a
/// crate with no UI dependency, where it can be tested — and the editor is
/// left with only the drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    pub id: NodeId,
    pub kind: Kind,
    /// Fractions of the output: x, y, width, height.
    pub rect: [f32; 4],
    pub label: String,
    pub value: f64,
    pub text: String,
    /// Slider ends, and whether it stands up.
    pub range: (f64, f64),
    pub vertical: bool,
    pub momentary: bool,
    pub display: bool,
    pub enabled: bool,
    pub opacity: f32,
    pub bg: [f32; 4],
    pub layer: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Button,
    Slider,
    Field,
    Container,
    Text,
    OpViewer,
    Network,
}

pub fn container_view_pixel_size(graph: &Graph, id: NodeId) -> (f32, f32) {
    let ctx = EvalContext::default();
    let node = graph.node(id);
    let w = node
        .param("vieww")
        .map(|p| p.eval(&ctx).as_f64())
        .unwrap_or(400.0) as f32;
    let h = node
        .param("viewh")
        .map(|p| p.eval(&ctx).as_f64())
        .unwrap_or(300.0) as f32;
    (w.clamp(64.0, 16384.0), h.clamp(64.0, 16384.0))
}

pub fn layout_rect(graph: &Graph, id: NodeId) -> Option<[f32; 4]> {
    crate::layout::resolve_panel_rects(graph)
        .get(&id)
        .copied()
}

pub fn panel_direct_display_widgets(graph: &Graph, comp: NodeId) -> Vec<Widget> {
    let comp_rect = layout_rect(graph, comp).unwrap_or([0.0, 0.0, 1.0, 1.0]);
    let (cx, cy, cw, ch) = (comp_rect[0], comp_rect[1], comp_rect[2], comp_rect[3]);
    if cw <= 0.0 || ch <= 0.0 {
        return Vec::new();
    }
    widgets(graph)
        .into_iter()
        .filter(|w| graph.get(w.id).and_then(|n| n.parent) == Some(comp))
        .map(|mut w| {
            let r = w.rect;
            w.rect = [(r[0] - cx) / cw, (r[1] - cy) / ch, r[2] / cw, r[3] / ch];
            w
        })
        .collect()
}

pub fn panel_display_widgets(graph: &Graph, comp: NodeId) -> Vec<Widget> {
    let Some(node) = graph.get(comp) else {
        return Vec::new();
    };
    if node.family != otd_core::Family::Comp || !is_panel_type(&node.op_type) {
        return Vec::new();
    }
    if node.op_type == CONTAINER {
        return panel_direct_display_widgets(graph, comp);
    }
    widgets(graph)
        .into_iter()
        .filter(|w| w.id == comp)
        .map(|mut w| {
            w.rect = [0.0, 0.0, 1.0, 1.0];
            w
        })
        .collect()
}

pub fn is_panel_surface_widget(graph: &Graph, id: NodeId) -> bool {
    let Some(node) = graph.get(id) else {
        return false;
    };
    if node.op_type != TEXT {
        return true;
    }
    let Some(parent) = node.parent else {
        return true;
    };
    let parent = graph.node(parent).op_type.as_str();
    !matches!(parent, BUTTON | SLIDER | FIELD)
}

/// Every widget in the project, in a stable order.
///
/// Sorted by path so two buttons never swap places between frames, which
/// would make one of them impossible to click.
pub fn widgets(graph: &Graph) -> Vec<Widget> {
    let rects = crate::layout::resolve_panel_rects(graph);
    let mut ids: Vec<NodeId> = graph
        .walk()
        .into_iter()
        .filter(|id| *id != graph.root())
        .filter(|id| graph.get(*id).is_some_and(|n| is_panel(&n.op_type)))
        .filter(|id| is_panel_surface_widget(graph, *id))
        .collect();
    ids.sort_by_key(|id| graph.path(*id));

    let ctx = otd_core::EvalContext::default();
    let mut out: Vec<Widget> = ids
        .into_iter()
        .filter_map(|id| {
            let node = graph.get(id)?;
            let f = |key: &str| {
                node.param(key)
                    .map(|p| p.eval(&ctx).as_f64())
                    .unwrap_or(0.0)
            };
            let b = |key: &str, default: bool| {
                node.param(key)
                    .map(|p| p.eval(&ctx).as_bool())
                    .unwrap_or(default)
            };
            let s = |key: &str| {
                node.param(key)
                    .map(|p| p.eval(&ctx).as_str())
                    .unwrap_or_default()
            };
            let kind = match node.op_type.as_str() {
                BUTTON => Kind::Button,
                SLIDER => Kind::Slider,
                CONTAINER => Kind::Container,
                TEXT => Kind::Text,
                OP_VIEWER => Kind::OpViewer,
                FIELD => Kind::Field,
                _ => Kind::Network,
            };
            if !panel_display_visible(graph, id, &ctx) {
                return None;
            }
            let display = b("display", true);
            let label = {
                let given = s("label");
                let factory = crate::panel::defs::factory_label_param(&node.op_type);
                if given.trim().is_empty() || (!factory.is_empty() && given == factory) {
                    node.name.clone()
                } else {
                    given
                }
            };
            let rect = rects.get(&id).copied().unwrap_or([0.05, 0.05, 0.2, 0.08]);
            Some(Widget {
                id,
                kind,
                rect,
                label,
                value: f("value"),
                text: s("text"),
                range: (f("min"), f("max")),
                vertical: s("orientation") == "vertical",
                momentary: s("mode") == "momentary",
                display,
                enabled: b("enable", true),
                opacity: f("opacity") as f32,
                bg: [
                    f("bgcolorr") as f32,
                    f("bgcolorg") as f32,
                    f("bgcolorb") as f32,
                    f("bgalpha") as f32,
                ],
                layer: f("layer") as f32,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a.layer
            .partial_cmp(&b.layer)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| graph.path(a.id).cmp(&graph.path(b.id)))
    });
    out
}

#[cfg(test)]
#[path = "../tests/panel__tests.rs"]
mod tests;
