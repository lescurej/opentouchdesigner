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

use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Graph, NodeId, OpDef, OpRegistry, Param};

pub const BUTTON: &str = "buttonCOMP";
pub const SLIDER: &str = "sliderCOMP";
pub const FIELD: &str = "fieldCOMP";
pub const CONTAINER: &str = "containerCOMP";
pub const TEXT: &str = "textCOMP";
pub const GLSL_PANEL: &str = "glslCOMP";
pub const LIST: &str = "listCOMP";
pub const OP_VIEWER: &str = "opviewerCOMP";
pub const PARAMETER: &str = "parameterCOMP";
pub const SELECT: &str = "selectCOMP";
pub const TABLE: &str = "tableCOMP";
pub const WIDGET: &str = "widgetCOMP";

const PANEL_TYPES: &[&str] = &[
    BUTTON,
    SLIDER,
    FIELD,
    CONTAINER,
    TEXT,
    GLSL_PANEL,
    LIST,
    OP_VIEWER,
    PARAMETER,
    SELECT,
    TABLE,
    WIDGET,
];

pub fn is_panel_type(op_type: &str) -> bool {
    PANEL_TYPES.contains(&op_type)
}

/// Whether an operator type is a widget this module draws.
pub fn is_panel(op_type: &str) -> bool {
    is_panel_type(op_type)
}

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}

fn finish_widget_params(mut m: IndexMap<String, Param>, layout: (f64, f64, f64, f64)) -> IndexMap<String, Param> {
    crate::panel_params::append_layout_page(&mut m, layout);
    crate::panel_params::append_panel_page_no_label(&mut m);
    crate::panel_params::append_look_page(&mut m);
    m
}

fn with_layout(m: IndexMap<String, Param>) -> IndexMap<String, Param> {
    finish_widget_params(m, (0.02, 0.02, 0.44, 0.12))
}

fn params_container() -> IndexMap<String, Param> {
    let mut m = params! {};
    crate::panel_params::append_layout_page(&mut m, (0.0, 0.0, 1.0, 1.0));
    crate::panel_params::append_panel_page(&mut m);
    crate::panel_params::append_look_page(&mut m);
    crate::panel_params::append_container_view_page(&mut m);
    crate::panel_params::append_container_children_page(&mut m);
    m
}

pub fn panel_display_visible(graph: &Graph, id: NodeId, ctx: &EvalContext) -> bool {
    let mut cur = Some(id);
    while let Some(cid) = cur {
        let Some(node) = graph.get(cid) else {
            break;
        };
        if is_panel_type(&node.op_type) {
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
    summary: "Parent panel for buttons, sliders, text, OP viewers, and nested containers; Children page drives align/layout.",
    time_dependent: false,
    params: params_container,
    connector: Connector::None,
};

fn params_button() -> IndexMap<String, Param> {
    with_layout(params! {
        "label" => Param::str("Button").with_label("Label").with_page("Button"),
        "value" => Param::float(0.0).with_label("Value").with_range(0.0, 1.0).with_page("Button"),
        "mode" => Param::menu("toggle", &["toggle", "momentary"])
            .with_label("Button Type")
            .with_page("Button"),
    })
}

fn params_slider() -> IndexMap<String, Param> {
    with_layout(params! {
        "label" => Param::str("Slider").with_label("Label").with_page("Slider"),
        "value" => Param::float(0.5).with_label("Value").with_page("Slider"),
        "min" => Param::float(0.0).with_label("Minimum").with_page("Slider"),
        "max" => Param::float(1.0).with_label("Maximum").with_page("Slider"),
        "orientation" => Param::menu("horizontal", &["horizontal", "vertical"])
            .with_label("Orientation")
            .with_page("Slider"),
    })
}

fn params_field() -> IndexMap<String, Param> {
    with_layout(params! {
        "text" => Param::str("").with_label("Text").with_page("Field"),
    })
}

fn params_text_comp() -> IndexMap<String, Param> {
    finish_widget_params(
        params! {
            "text" => Param::str("").with_label("Text").with_page("Text"),
            "fontsize" => Param::float(0.5)
                .with_label("Font Size")
                .with_range(0.05, 2.0)
                .with_page("Text"),
        },
        (0.05, 0.05, 0.2, 0.08),
    )
}

fn params_op_viewer() -> IndexMap<String, Param> {
    with_layout(params! {
        "op" => Param::str("").with_label("Operator"),
    })
}

fn stub_panel(type_name: &'static str, label: &'static str, summary: &'static str) -> OpDef {
    OpDef {
        type_name,
        input_families: &[],
        label,
        family: Family::Comp,
        inputs: &[],
        summary,
        time_dependent: false,
        params: || with_layout(params! {}),
        connector: Connector::None,
    }
}

pub fn defs() -> Vec<OpDef> {
    vec![
        OpDef {
            type_name: BUTTON,
            input_families: &[],
            label: "Button",
            family: Family::Comp,
            inputs: &[],
            summary: "Toggle or momentary control; interior textCOMP + textTOP draws the label.",
            time_dependent: false,
            params: params_button,
            connector: Connector::None,
        },
        CONTAINER_DEF,
        OpDef {
            type_name: FIELD,
            input_families: &[],
            label: "Field",
            family: Family::Comp,
            inputs: &[],
            summary: "Editable string parameter with textCOMP/textTOP interior.",
            time_dependent: false,
            params: params_field,
            connector: Connector::None,
        },
        stub_panel(
            GLSL_PANEL,
            "GLSL",
            "Panel region filled from an interior glslTOP network (layout only until cook hook lands).",
        ),
        stub_panel(
            LIST,
            "List",
            "Scrollable list panel — parameters match TD; rendering uses interior TOPs.",
        ),
        OpDef {
            type_name: OP_VIEWER,
            input_families: &[],
            label: "OP Viewer",
            family: Family::Comp,
            inputs: &[],
            summary: "Shows another operator's cooked TOP inside this panel rect.",
            time_dependent: false,
            params: params_op_viewer,
            connector: Connector::None,
        },
        stub_panel(
            PARAMETER,
            "Parameter",
            "Auto-layout of another operator's custom parameters (stub — build from par + TOP).",
        ),
        stub_panel(
            SELECT,
            "Select",
            "Displays another panel COMP by path (stub — use opviewer + container for now).",
        ),
        OpDef {
            type_name: SLIDER,
            input_families: &[],
            label: "Slider",
            family: Family::Comp,
            inputs: &[],
            summary: "Fader control; Value parameter is the state the network reads.",
            time_dependent: false,
            params: params_slider,
            connector: Connector::None,
        },
        stub_panel(
            TABLE,
            "Table",
            "Tabular panel tied to a tableDAT (stub — interior TOP/DAT wiring).",
        ),
        OpDef {
            type_name: TEXT,
            input_families: &[],
            label: "Text",
            family: Family::Comp,
            inputs: &[],
            summary: "Static or expression-driven text via interior textTOP.",
            time_dependent: false,
            params: params_text_comp,
            connector: Connector::None,
        },
        stub_panel(
            WIDGET,
            "Widget",
            "Reusable panel asset hook (stub — same graph-in-COMP model as other panels).",
        ),
    ]
}

pub fn register(registry: &mut OpRegistry) {
    for def in defs() {
        registry.register(def);
    }
}

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
    (
        w.clamp(64.0, 16384.0),
        h.clamp(64.0, 16384.0),
    )
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
            w.rect = [
                (r[0] - cx) / cw,
                (r[1] - cy) / ch,
                r[2] / cw,
                r[3] / ch,
            ];
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
    let rects = crate::panel_layout::resolve_panel_rects(graph);
    let mut ids: Vec<NodeId> = graph
        .walk()
        .into_iter()
        .filter(|id| *id != graph.root())
        .filter(|id| graph.get(*id).is_some_and(|n| is_panel(&n.op_type)))
        .filter(|id| is_panel_surface_widget(graph, *id))
        .collect();
    ids.sort_by_key(|id| graph.path(*id));

    let ctx = otd_core::EvalContext::default();
    let mut out: Vec<Widget> = ids.into_iter()
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
                if given.trim().is_empty() {
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
mod tests {
    use super::*;
    use otd_core::Value;

    fn graph_with(op: &str, name: &str) -> (Graph, NodeId) {
        let mut reg = OpRegistry::new();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        let id = graph
            .create(root, reg.get(op).unwrap(), Some(name))
            .unwrap();
        (graph, id)
    }

    #[test]
    fn a_widget_reads_its_layout_and_state_from_parameters() {
        let (mut graph, id) = graph_with(SLIDER, "fader1");
        graph.set_param(id, "x", Value::Float(0.25)).unwrap();
        graph.set_param(id, "value", Value::Float(0.7)).unwrap();
        graph.set_param(id, "max", Value::Float(4.0)).unwrap();

        let w = widgets(&graph);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].kind, Kind::Slider);
        assert_eq!(w[0].rect[0], 0.25);
        assert_eq!(w[0].value, 0.7);
        assert_eq!(w[0].range, (0.0, 4.0));
    }

    #[test]
    fn an_unlabelled_widget_shows_its_node_name() {
        let (graph, _) = graph_with(BUTTON, "go");
        assert_eq!(widgets(&graph)[0].label, "go");
    }

    #[test]
    fn interior_text_comp_is_not_a_separate_widget() {
        let reg = crate::registry();
        let mut graph = Graph::new();
        let root = graph.root();
        let btn = graph
            .create(root, reg.get(BUTTON).unwrap(), Some("button1"))
            .unwrap();
        crate::panel_templates::install_button(&mut graph, &reg, btn);
        let w = widgets(&graph);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].id, btn);
    }

    #[test]
    fn widgets_keep_a_stable_order_so_they_stay_clickable() {
        let mut reg = OpRegistry::new();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        // Created out of order on purpose.
        for name in ["zed", "alpha", "mid"] {
            graph
                .create(root, reg.get(BUTTON).unwrap(), Some(name))
                .unwrap();
        }
        let names: Vec<String> = widgets(&graph).iter().map(|w| w.label.clone()).collect();
        assert_eq!(names, vec!["alpha", "mid", "zed"]);
    }

    #[test]
    fn container_view_defaults_to_400_by_300() {
        let mut reg = OpRegistry::new();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        let id = graph
            .create(root, reg.get(CONTAINER).unwrap(), Some("ui"))
            .unwrap();
        let (w, h) = container_view_pixel_size(&graph, id);
        assert_eq!(w, 400.0);
        assert_eq!(h, 300.0);
    }

    #[test]
    fn container_display_is_direct_children_only() {
        let mut reg = OpRegistry::new();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        let outer = graph
            .create(root, reg.get(CONTAINER).unwrap(), Some("outer"))
            .unwrap();
        let inner = graph
            .create(outer, reg.get(CONTAINER).unwrap(), Some("inner"))
            .unwrap();
        let btn = graph
            .create(inner, reg.get(BUTTON).unwrap(), Some("go"))
            .unwrap();
        let outer_w = panel_display_widgets(&graph, outer);
        assert_eq!(outer_w.len(), 1);
        assert_eq!(outer_w[0].id, inner);
        assert!(outer_w.iter().all(|w| w.id != btn));
        let inner_w = panel_display_widgets(&graph, inner);
        assert_eq!(inner_w.len(), 1);
        assert_eq!(inner_w[0].id, btn);
    }

    #[test]
    fn nothing_but_a_panel_comp_is_a_widget() {
        let mut reg = otd_gpu::ops::registry();
        register(&mut reg);
        let mut graph = Graph::new();
        let root = graph.root();
        graph
            .create(root, reg.get("noiseTOP").unwrap(), Some("noise1"))
            .unwrap();
        graph
            .create(root, reg.get(BUTTON).unwrap(), Some("go"))
            .unwrap();
        assert_eq!(widgets(&graph).len(), 1);
    }
}
