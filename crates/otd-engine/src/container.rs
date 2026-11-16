//! `containerCOMP` — panel surface that lays out child COMPs and composites their TOPs.
//!
//! Widget COMPs (button, slider, …) were removed; children are typically `baseCOMP`
//! networks whose cooked output is shown in the layout rect. Layout still uses
//! normalized fractions and the Children/View parameter pages from TouchDesigner.

use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Graph, NodeId, OpDef, OpRegistry, Param};

pub const CONTAINER: &str = "containerCOMP";

pub fn is_container_type(op_type: &str) -> bool {
    op_type == CONTAINER
}

/// `baseCOMP` in the panel tree under a `containerCOMP` (direct child or nested
/// inside other panel `baseCOMP`s, including clone instances).
pub fn is_panel_child_base_comp(graph: &Graph, id: NodeId) -> bool {
    let Some(node) = graph.get(id) else {
        return false;
    };
    if !crate::comp::is_base_type(&node.op_type) {
        return false;
    }
    let mut cur = node.parent;
    while let Some(pid) = cur {
        let Some(parent) = graph.get(pid) else {
            break;
        };
        if parent.op_type == CONTAINER {
            return true;
        }
        if parent.family != Family::Comp {
            return false;
        }
        if !crate::comp::is_base_type(&parent.op_type) {
            return false;
        }
        cur = parent.parent;
    }
    false
}

/// Operators that participate in container layout (container itself or a direct child COMP).
pub fn participates_in_layout(graph: &Graph, id: NodeId) -> bool {
    otd_panel::participates_in_layout(graph, id)
}

fn params_container() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    otd_panel::append_layout_page(&mut m, (0.0, 0.0, 1.0, 1.0));
    otd_panel::append_panel_page(&mut m);
    otd_panel::append_look_page(&mut m);
    otd_panel::append_container_view_page(&mut m);
    otd_panel::append_container_children_page(&mut m);
    m
}

pub fn display_visible(graph: &Graph, id: NodeId, ctx: &EvalContext) -> bool {
    let mut cur = Some(id);
    while let Some(cid) = cur {
        let Some(node) = graph.get(cid) else {
            break;
        };
        if participates_in_layout(graph, cid) {
            let show = node
                .param("display")
                .map(|p| p.eval(ctx).as_bool())
                .unwrap_or(true);
            if !show {
                return false;
            }
        }
        cur = node.parent;
    }
    true
}

pub const CONTAINER_DEF: OpDef = OpDef {
    type_name: CONTAINER,
    input_families: &[],
    label: "Container",
    family: Family::Comp,
    inputs: &[],
    summary: "Panel surface that lays out child COMPs and draws their cooked TOPs; Children page drives align/layout.",
    time_dependent: false,
    params: params_container,
    connector: Connector::None,
};

pub fn register(registry: &mut OpRegistry) {
    registry.register(CONTAINER_DEF);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    pub id: NodeId,
    pub kind: Kind,
    /// Fractions of the output: x, y, width, height.
    pub rect: [f32; 4],
    pub label: String,
    pub display: bool,
    pub enabled: bool,
    pub opacity: f32,
    pub bg: [f32; 4],
    pub layer: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Container,
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
    crate::panel_layout::resolve_panel_rects(graph)
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
    if node.family != Family::Comp || !is_container_type(&node.op_type) {
        return Vec::new();
    }
    panel_direct_display_widgets(graph, comp)
}

pub fn widgets(graph: &Graph) -> Vec<Widget> {
    let rects = crate::panel_layout::resolve_panel_rects(graph);
    let mut ids: Vec<NodeId> = graph
        .walk()
        .into_iter()
        .filter(|id| *id != graph.root())
        .filter(|id| participates_in_layout(graph, *id))
        .collect();
    ids.sort_by_key(|id| graph.path(*id));

    let ctx = EvalContext::default();
    let mut out: Vec<Widget> = ids
        .into_iter()
        .filter_map(|id| {
            let node = graph.get(id)?;
            if !display_visible(graph, id, &ctx) {
                return None;
            }
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
            let kind = if node.op_type == CONTAINER {
                Kind::Container
            } else {
                Kind::Network
            };
            let display = b("display", true);
            let label = node.name.clone();
            let rect = rects.get(&id).copied().unwrap_or([0.05, 0.05, 0.2, 0.08]);
            Some(Widget {
                id,
                kind,
                rect,
                label,
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
#[path = "tests/container__tests.rs"]
mod tests;
