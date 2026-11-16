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
    BUTTON, SLIDER, FIELD, CONTAINER, TEXT, GLSL_PANEL, LIST, OP_VIEWER, PARAMETER, SELECT, TABLE,
    WIDGET,
];

pub fn is_panel_type(op_type: &str) -> bool {
    PANEL_TYPES.contains(&op_type)
}

/// Whether an operator type is a widget this module draws.
pub fn is_panel(op_type: &str) -> bool {
    is_panel_type(op_type)
}

pub fn factory_label_param(op_type: &str) -> &'static str {
    match op_type {
        BUTTON => "Button",
        SLIDER => "Slider",
        _ => "",
    }
}

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}

fn finish_widget_params(
    mut m: IndexMap<String, Param>,
    layout: (f64, f64, f64, f64),
) -> IndexMap<String, Param> {
    crate::params::append_layout_page(&mut m, layout);
    crate::params::append_panel_page_no_label(&mut m);
    crate::params::append_look_page(&mut m);
    m
}

fn with_layout(m: IndexMap<String, Param>) -> IndexMap<String, Param> {
    finish_widget_params(m, (0.02, 0.02, 0.44, 0.12))
}

fn params_container() -> IndexMap<String, Param> {
    let mut m = params! {};
    crate::params::append_layout_page(&mut m, (0.0, 0.0, 1.0, 1.0));
    crate::params::append_panel_page(&mut m);
    crate::params::append_look_page(&mut m);
    crate::params::append_container_view_page(&mut m);
    crate::params::append_container_children_page(&mut m);
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
