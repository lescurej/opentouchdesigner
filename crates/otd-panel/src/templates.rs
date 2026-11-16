use otd_core::{Graph, NodeId, OpRegistry, Value};

use crate::panel as panel;

const BUTTON_PANELEXEC: &str = "\
def onOffToOn(panelValue):
    setpar('.', 'value', 1.0)

def onOnToOff(panelValue):
    setpar('.', 'value', 0.0)
";

fn add(graph: &mut Graph, reg: &OpRegistry, parent: NodeId, op: &str, name: &str) -> NodeId {
    let def = reg
        .get(op)
        .unwrap_or_else(|| panic!("panel template needs `{op}`"))
        .clone();
    graph.create(parent, &def, Some(name)).unwrap()
}

fn place(graph: &mut Graph, id: NodeId, x: f32, y: f32) {
    graph.node_mut_quiet(id).pos = [x, y];
}

fn tune_text_top(graph: &mut Graph, top: NodeId) {
    let _ = graph.set_param(top, "color", Value::Vec4([0.95, 0.96, 0.98, 1.0]));
    let _ = graph.set_param(top, "size", Value::Float(20.0));
}

fn text_comp_out_top(graph: &Graph, comp: NodeId) -> Option<NodeId> {
    let text = child_named(graph, comp, "text")?;
    child_named(graph, text, "out1").or_else(|| {
        graph
            .children(text)
            .iter()
            .find(|id| graph.node(**id).family == otd_core::Family::Top)
            .copied()
    })
}

fn child_named(graph: &Graph, parent: NodeId, name: &str) -> Option<NodeId> {
    graph
        .children(parent)
        .iter()
        .find(|id| graph.node(**id).name == name)
        .copied()
}

pub fn install_text_comp_interior(graph: &mut Graph, reg: &OpRegistry, comp: NodeId) {
    if let Some(top) = graph
        .children(comp)
        .iter()
        .find(|id| graph.node(**id).op_type == "textTOP")
    {
        tune_text_top(graph, *top);
        return;
    }
    let top = add(graph, reg, comp, "textTOP", "text1");
    graph.set_expression(top, "text", "parent.text").unwrap();
    tune_text_top(graph, top);
    graph.node_mut_quiet(top).flags.viewer = true;

    let out = add(graph, reg, comp, "outTOP", "out1");
    graph.connect(top, out, 0).unwrap();

    place(graph, top, -120.0, 0.0);
    place(graph, out, 120.0, 0.0);
}

fn remove_legacy_direct_text_tops(graph: &mut Graph, parent: NodeId) {
    let legacy: Vec<NodeId> = graph
        .children(parent)
        .iter()
        .copied()
        .filter(|id| graph.node(*id).op_type == "textTOP")
        .collect();
    for id in legacy {
        let _ = graph.remove(id);
    }
}

fn ensure_text_comp(
    graph: &mut Graph,
    reg: &OpRegistry,
    parent: NodeId,
    text_expr: &str,
) -> NodeId {
    remove_legacy_direct_text_tops(graph, parent);
    if let Some(id) =
        child_named(graph, parent, "text").filter(|id| graph.node(*id).op_type == panel::TEXT)
    {
        let _ = graph.set_expression(id, "text", text_expr);
        install_text_comp_interior(graph, reg, id);
        return id;
    }
    let comp = add(graph, reg, parent, panel::TEXT, "text");
    graph.set_param(comp, "w", Value::Float(1.0)).unwrap();
    graph.set_param(comp, "h", Value::Float(1.0)).unwrap();
    graph
        .set_param(comp, "fontsize", Value::Float(0.5))
        .unwrap();
    graph.set_expression(comp, "text", text_expr).unwrap();
    install_text_comp_interior(graph, reg, comp);
    if let Some(top) = graph
        .children(comp)
        .iter()
        .find(|id| graph.node(**id).op_type == "textTOP")
    {
        tune_text_top(graph, *top);
    }
    comp
}

pub fn ensure_button_interior(graph: &mut Graph, reg: &OpRegistry, comp: NodeId) {
    if graph.node(comp).op_type != panel::BUTTON {
        return;
    }
    if graph
        .node(comp)
        .params
        .get("label")
        .is_some_and(|p| p.value.as_str().trim().is_empty())
    {
        let _ = graph.set_param(comp, "label", Value::Str("Button".into()));
    }

    let par1 = child_named(graph, comp, "par1").unwrap_or_else(|| {
        let id = add(graph, reg, comp, "parameterCHOP", "par1");
        graph.set_param(id, "op", Value::Str("..".into())).unwrap();
        graph
            .set_param(id, "parameters", Value::Str("value".into()))
            .unwrap();
        graph.set_param(id, "name", Value::Str("v".into())).unwrap();
        place(graph, id, -280.0, 80.0);
        id
    });

    let out_chop = child_named(graph, comp, "out1").unwrap_or_else(|| {
        let id = add(graph, reg, comp, "outCHOP", "out1");
        place(graph, id, 80.0, 80.0);
        id
    });
    if graph.resolve_input_source(out_chop, 0) != Some(par1) {
        let _ = graph.connect(par1, out_chop, 0);
    }

    let text = ensure_text_comp(graph, reg, comp, "parent.label");
    place(graph, text, -280.0, -60.0);

    let _ = child_named(graph, comp, "parexec1").unwrap_or_else(|| {
        let id = add(graph, reg, comp, "parameterexecuteDAT", "parexec1");
        graph.set_param(id, "op", Value::Str("..".into())).unwrap();
        graph
            .set_param(id, "parameters", Value::Str("value".into()))
            .unwrap();
        place(graph, id, -280.0, -200.0);
        id
    });

    let _ = child_named(graph, comp, "panelexec1").unwrap_or_else(|| {
        let id = add(graph, reg, comp, "panelexecuteDAT", "panelexec1");
        graph
            .set_param(id, "panels", Value::Str("..".into()))
            .unwrap();
        graph
            .set_param(id, "panelvalue", Value::Str("value".into()))
            .unwrap();
        graph
            .set_param(id, "valuechange", Value::Bool(false))
            .unwrap();
        graph
            .set_param(id, "source", Value::Str(BUTTON_PANELEXEC.into()))
            .unwrap();
        place(graph, id, 80.0, -200.0);
        id
    });

    ensure_button_panel_out(graph, reg, comp);
}

fn ensure_button_panel_out(graph: &mut Graph, reg: &OpRegistry, comp: NodeId) {
    if child_named(graph, comp, "view1").is_some() {
        return;
    }
    let Some(text_out) = text_comp_out_top(graph, comp) else {
        return;
    };

    let bg = add(graph, reg, comp, "constantTOP", "bg1");
    let _ = graph.set_param(bg, "color", Value::Vec4([0.22, 0.24, 0.30, 1.0]));

    let panel = add(graph, reg, comp, "compositeTOP", "panel1");
    let _ = graph.connect(bg, panel, 0);
    let _ = graph.connect(text_out, panel, 1);

    let view = add(graph, reg, comp, "outTOP", "view1");
    let _ = graph.connect(panel, view, 0);

    place(graph, bg, -120.0, 120.0);
    place(graph, panel, 0.0, 120.0);
    place(graph, view, 120.0, 120.0);
}

fn ensure_slider_panel_out(graph: &mut Graph, reg: &OpRegistry, comp: NodeId) {
    if child_named(graph, comp, "view1").is_some() {
        return;
    }
    let Some(text_out) = text_comp_out_top(graph, comp) else {
        return;
    };

    let track = add(graph, reg, comp, "constantTOP", "track1");
    let _ = graph.set_param(track, "color", Value::Vec4([0.12, 0.13, 0.16, 1.0]));

    let fill_src = add(graph, reg, comp, "constantTOP", "fill1");
    let _ = graph.set_param(fill_src, "color", Value::Vec4([0.35, 0.58, 0.95, 1.0]));

    let fill_crop = add(graph, reg, comp, "cropTOP", "fillcrop1");
    let _ = graph.connect(fill_src, fill_crop, 0);
    let _ = graph.set_param(fill_crop, "left", Value::Float(0.0));
    let _ = graph.set_param(fill_crop, "bottom", Value::Float(0.0));
    let _ = graph.set_param(fill_crop, "top", Value::Float(1.0));
    let _ = graph.set_expression(
        fill_crop,
        "right",
        "((parent.value - parent.min) / max(parent.max - parent.min, 0.001))",
    );

    let bar = add(graph, reg, comp, "compositeTOP", "bar1");
    let _ = graph.connect(track, bar, 0);
    let _ = graph.connect(fill_crop, bar, 1);

    let panel = add(graph, reg, comp, "compositeTOP", "panel1");
    let _ = graph.connect(bar, panel, 0);
    let _ = graph.connect(text_out, panel, 1);

    let view = add(graph, reg, comp, "outTOP", "view1");
    let _ = graph.connect(panel, view, 0);

    place(graph, track, -160.0, 120.0);
    place(graph, fill_src, -80.0, 120.0);
    place(graph, fill_crop, 0.0, 120.0);
    place(graph, bar, 80.0, 120.0);
    place(graph, panel, 160.0, 120.0);
    place(graph, view, 240.0, 120.0);
}

pub fn ensure_field_interior(graph: &mut Graph, reg: &OpRegistry, comp: NodeId) {
    if graph.node(comp).op_type != panel::FIELD {
        return;
    }
    let _ = ensure_text_comp(graph, reg, comp, "parent.text");
}

pub fn ensure_slider_interior(graph: &mut Graph, reg: &OpRegistry, comp: NodeId) {
    if graph.node(comp).op_type != panel::SLIDER {
        return;
    }
    if graph
        .node(comp)
        .params
        .get("label")
        .is_some_and(|p| p.value.as_str().trim().is_empty())
    {
        let _ = graph.set_param(comp, "label", Value::Str("Slider".into()));
    }

    let par1 = child_named(graph, comp, "par1").unwrap_or_else(|| {
        let id = add(graph, reg, comp, "parameterCHOP", "par1");
        graph.set_param(id, "op", Value::Str("..".into())).unwrap();
        graph
            .set_param(id, "parameters", Value::Str("value".into()))
            .unwrap();
        graph.set_param(id, "name", Value::Str("v".into())).unwrap();
        place(graph, id, -280.0, 80.0);
        id
    });

    let out_chop = child_named(graph, comp, "out1").unwrap_or_else(|| {
        let id = add(graph, reg, comp, "outCHOP", "out1");
        place(graph, id, 80.0, 80.0);
        id
    });
    if graph.resolve_input_source(out_chop, 0) != Some(par1) {
        let _ = graph.connect(par1, out_chop, 0);
    }

    let text = ensure_text_comp(graph, reg, comp, "parent.label");
    place(graph, text, -280.0, -60.0);

    let _ = child_named(graph, comp, "parexec1").unwrap_or_else(|| {
        let id = add(graph, reg, comp, "parameterexecuteDAT", "parexec1");
        graph.set_param(id, "op", Value::Str("..".into())).unwrap();
        graph
            .set_param(id, "parameters", Value::Str("value min max".into()))
            .unwrap();
        place(graph, id, -280.0, -200.0);
        id
    });

    let _ = child_named(graph, comp, "panelexec1").unwrap_or_else(|| {
        let id = add(graph, reg, comp, "panelexecuteDAT", "panelexec1");
        graph
            .set_param(id, "panels", Value::Str("..".into()))
            .unwrap();
        graph
            .set_param(id, "panelvalue", Value::Str("value".into()))
            .unwrap();
        graph.set_param(id, "offtoon", Value::Bool(false)).unwrap();
        graph.set_param(id, "ontooff", Value::Bool(false)).unwrap();
        place(graph, id, 80.0, -200.0);
        id
    });

    ensure_slider_panel_out(graph, reg, comp);
}

pub fn upgrade_panel_internals(graph: &mut Graph, reg: &OpRegistry) {
    for id in graph.walk() {
        match graph.node(id).op_type.as_str() {
            panel::BUTTON => ensure_button_interior(graph, reg, id),
            panel::SLIDER => ensure_slider_interior(graph, reg, id),
            panel::FIELD => ensure_field_interior(graph, reg, id),
            panel::TEXT => install_text_comp_interior(graph, reg, id),
            _ => {}
        }
    }
}

pub fn install_button(graph: &mut Graph, reg: &OpRegistry, comp: NodeId) {
    ensure_button_interior(graph, reg, comp);
}

pub fn install_slider(graph: &mut Graph, reg: &OpRegistry, comp: NodeId) {
    ensure_slider_interior(graph, reg, comp);
}

pub fn install_if_panel(graph: &mut Graph, reg: &OpRegistry, comp: NodeId, op_type: &str) {
    match op_type {
        panel::BUTTON => ensure_button_interior(graph, reg, comp),
        panel::SLIDER => ensure_slider_interior(graph, reg, comp),
        panel::FIELD => ensure_field_interior(graph, reg, comp),
        panel::TEXT => install_text_comp_interior(graph, reg, comp),
        _ => {}
    }
}
