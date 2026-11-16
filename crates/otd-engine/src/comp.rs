use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, Graph, NodeId, OpDef, OpRegistry, Param};

pub const BASE: &str = "baseCOMP";

fn params_base() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "w".into(),
        Param::float(0.2).with_label("Width").with_page("Layout"),
    );
    m.insert(
        "h".into(),
        Param::float(0.08).with_label("Height").with_page("Layout"),
    );
    m.insert(
        "top".into(),
        Param::str("")
            .with_label("Background TOP")
            .with_page("Look")
            .as_path_ref(),
    );
    m.insert(
        "borderover".into(),
        Param::bool(false)
            .with_label("Border Over Children")
            .with_page("Look"),
    );
    m.insert(
        "parentshortcut".into(),
        Param::str("")
            .with_label("Parent Shortcut")
            .with_page("Common"),
    );
    m.insert(
        "displaymode".into(),
        Param::menu("default", &["default", "inside", "viewer", "mixed"])
            .with_label("Display Mode")
            .with_page("Panel"),
    );
    m
}

fn base_def(type_name: &'static str) -> OpDef {
    OpDef {
        type_name,
        input_families: &[],
        label: "Base",
        family: Family::Comp,
        inputs: &[],
        summary: "Sub-network; In and Out operators inside define inputs and outputs.",
        time_dependent: false,
        params: params_base,
        connector: Connector::None,
    }
}

fn missing_comp_def() -> OpDef {
    OpDef {
        type_name: otd_core::MISSING_COMP,
        input_families: &[],
        label: "Missing",
        family: Family::Comp,
        inputs: &[],
        summary: "Placeholder for an unknown or removed component type.",
        time_dependent: false,
        params: otd_core::missing::missing_params,
        connector: Connector::None,
    }
}

pub fn register_all(registry: &mut OpRegistry) {
    registry.register(base_def(BASE));
    registry.register(missing_comp_def());
}

pub fn is_base_type(op_type: &str) -> bool {
    matches!(op_type, BASE)
}

/// Network output connectors on a COMP (Out ops inside the component), or
/// the single output tab every other operator presents on the canvas.
pub fn network_output_port_count(graph: &Graph, id: NodeId) -> usize {
    let node = graph.node(id);
    if node.family == Family::Comp {
        return graph.connector_ops(id, Connector::Out).len();
    }
    1
}

pub fn network_output_port_family(graph: &Graph, id: NodeId, index: usize) -> Option<Family> {
    graph
        .connector_ops(id, Connector::Out)
        .get(index)
        .map(|&out| graph.node(out).family)
}
