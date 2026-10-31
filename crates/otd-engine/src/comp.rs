use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, OpRegistry, Param};

pub const BASE: &str = "baseCOMP";

const LEGACY_ROOT: &str = "container";

fn params_none() -> IndexMap<String, Param> {
    IndexMap::new()
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
        params: params_none,
        connector: Connector::None,
    }
}

pub fn register(registry: &mut OpRegistry) {
    registry.register(base_def(BASE));
}

pub fn normalize_op_type(op: &str) -> &str {
    if op == LEGACY_ROOT { BASE } else { op }
}

pub fn is_base_type(op_type: &str) -> bool {
    matches!(op_type, BASE | LEGACY_ROOT)
}
