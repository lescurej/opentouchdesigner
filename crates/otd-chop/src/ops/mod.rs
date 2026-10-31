//! The CHOP operator table.
//!
//! Same shape as the TOP table in `otd-gpu`: one cook function and one entry
//! per operator, so breadth stays cheap (PLAN.md §6).
//!
//! Layout: `helpers`, `operators` (params + cook), `table` (registry entries).
//! Device I/O operators live in `crate::io` and are appended in `table`.

use std::sync::OnceLock;

use otd_core::indexmap::IndexMap;
use otd_core::{
    Connector, CookContext, Crossings, EvalContext, Family, Node, OpDef, OpRegistry, Param, Value,
};

#[macro_use]
mod helpers;
mod operators;
mod table;

pub use operators::{CONSTANT, CONSTANT_MAX_ROWS, INFO, PARAMETER};

use crate::data::ChopData;
use helpers::no_params;
use operators::cook_null;

/// Per-node scratch that survives between cooks.
#[derive(Clone, Debug, Default)]
pub struct OpState {
    pub f: Vec<f32>,
    pub i: Vec<i64>,
    pub prev: Vec<f32>,
}

impl OpState {
    pub(crate) fn fit(&mut self, n: usize) {
        self.f.resize(n, 0.0);
        self.i.resize(n, 0);
        self.prev.resize(n, 0.0);
    }
}

pub struct ChopCtx<'a> {
    pub graph: &'a otd_core::Graph,
    pub id: otd_core::NodeId,
    pub node: &'a Node,
    pub eval: &'a EvalContext<'a>,
    pub time: &'a CookContext,
    pub inputs: Vec<crate::data::ChopData>,
    pub foreign: Crossings,
    pub state: &'a mut OpState,
    pub io: &'a mut crate::io::Io,
    pub path: &'a str,
}

impl ChopCtx<'_> {
    pub fn val(&self, key: &str) -> Value {
        self.node
            .param(key)
            .map(|p| p.eval(self.eval))
            .unwrap_or(Value::Float(0.0))
    }
    pub fn f(&self, key: &str) -> f32 {
        self.val(key).as_f32()
    }
    pub fn i(&self, key: &str) -> i64 {
        self.val(key).as_i64()
    }
    pub fn b(&self, key: &str) -> bool {
        self.val(key).as_bool()
    }
    pub fn s(&self, key: &str) -> String {
        self.val(key).as_str()
    }
    pub fn menu(&self, key: &str) -> usize {
        let Some(param) = self.node.param(key) else {
            return 0;
        };
        let chosen = param.eval(self.eval).as_str();
        param
            .menu
            .as_ref()
            .and_then(|items| items.iter().position(|i| *i == chosen))
            .unwrap_or(0)
    }
    pub fn foreign(&self, index: usize) -> Option<&otd_core::Crossing> {
        self.foreign.get(index).and_then(|c| c.as_ref())
    }
    pub fn input(&self, index: usize) -> &crate::data::ChopData {
        static EMPTY: OnceLock<crate::data::ChopData> = OnceLock::new();
        self.inputs
            .get(index)
            .unwrap_or_else(|| EMPTY.get_or_init(crate::data::ChopData::empty))
    }
    pub fn rate(&self) -> f64 {
        let r = self.f("rate") as f64;
        if r > 0.0 { r } else { crate::data::CONTROL_RATE }
    }
}

pub struct ChopSpec {
    pub def: OpDef,
    pub cook: fn(&mut ChopCtx) -> crate::data::ChopData,
}

pub const NULL: &str = "nullCHOP";
pub const IN: &str = "inCHOP";
pub const OUT: &str = "outCHOP";

fn specs() -> &'static Vec<ChopSpec> {
    static SPECS: OnceLock<Vec<ChopSpec>> = OnceLock::new();
    SPECS.get_or_init(table::core_specs)
}

/// Small helper so the table above reads as a table.
pub(crate) fn spec(
    type_name: &'static str,
    label: &'static str,
    inputs: &'static [&'static str],
    summary: &'static str,
    params: fn() -> IndexMap<String, Param>,
    cook: fn(&mut ChopCtx) -> ChopData,
) -> ChopSpec {
    ChopSpec {
        def: OpDef {
            type_name,
            label,
            family: Family::Chop,
            inputs,
            input_families: &[],
            summary,
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        cook,
    }
}

/// The In and Out operators that give a component its channel connectors.
/// An In has no wire of its own; the engine fills it from outside.
pub(crate) fn connector_spec(
    type_name: &'static str,
    label: &'static str,
    inputs: &'static [&'static str],
    summary: &'static str,
    connector: Connector,
) -> ChopSpec {
    let mut s = spec(type_name, label, inputs, summary, no_params, cook_null);
    s.def.connector = connector;
    s
}

/// A converter: one input, of somebody else's family.
///
/// The input is called `in` like any other, because from the patch's point of
/// view it is one — the only thing that differs is which wires will land on
/// it, and the graph enforces that from [`OpDef::input_families`].
pub(crate) fn converter_spec(
    type_name: &'static str,
    label: &'static str,
    from: Family,
    summary: &'static str,
    params: fn() -> IndexMap<String, Param>,
    cook: fn(&mut ChopCtx) -> ChopData,
) -> ChopSpec {
    let mut s = spec(type_name, label, &["in"], summary, params, cook);
    s.def.input_families = match from {
        Family::Dat => &[Family::Dat],
        Family::Sop => &[Family::Sop],
        Family::Top => &[Family::Top],
        Family::Mat => &[Family::Mat],
        Family::Comp => &[Family::Comp],
        Family::Chop => &[Family::Chop],
    };
    s
}

/// Same, for operators that must cook every frame — generators over time and
/// anything reading a device.
pub(crate) fn spec_animated(
    type_name: &'static str,
    label: &'static str,
    inputs: &'static [&'static str],
    summary: &'static str,
    params: fn() -> IndexMap<String, Param>,
    cook: fn(&mut ChopCtx) -> ChopData,
) -> ChopSpec {
    let mut s = spec(type_name, label, inputs, summary, params, cook);
    s.def.time_dependent = true;
    s
}

pub fn spec_for(type_name: &str) -> Option<&'static ChopSpec> {
    specs().iter().find(|s| s.def.type_name == type_name)
}

pub fn all() -> impl Iterator<Item = &'static ChopSpec> {
    specs().iter()
}

pub fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    for s in specs() {
        r.register(s.def.clone());
    }
    r
}
