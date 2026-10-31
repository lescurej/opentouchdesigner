//! The DAT operator table.
//!
//! Layout: `helpers`, `operators`, `constants`, `table`.

use std::sync::OnceLock;

use otd_core::indexmap::IndexMap;
use otd_core::{
    Connector, Crossings, EvalContext, Family, Node, OpDef, OpRegistry, Param, Value,
};

use crate::{DatData, ScriptHost};

#[macro_use]
mod helpers;
mod operators;
mod constants;
mod table;

pub use constants::*;

pub struct DatCtx<'a> {
    pub graph: &'a otd_core::Graph,
    pub id: otd_core::NodeId,
    pub node: &'a Node,
    pub eval: &'a EvalContext<'a>,
    pub inputs: Vec<DatData>,
    pub foreign: Crossings,
    pub scripts: Option<&'a dyn ScriptHost>,
    pub net: &'a mut crate::net::Net,
    pub watched: &'a mut Watched,
    pub path: &'a str,
    pub error: Option<String>,
}

impl DatCtx<'_> {
    pub(crate) fn val(&self, key: &str) -> Value {
        self.node
            .param(key)
            .map(|p| p.eval(self.eval))
            .unwrap_or(Value::Str(String::new()))
    }
    pub(crate) fn s(&self, key: &str) -> String {
        self.val(key).as_str()
    }
    pub(crate) fn b(&self, key: &str) -> bool {
        self.val(key).as_bool()
    }
    pub(crate) fn menu(&self, key: &str) -> usize {
        let Some(p) = self.node.param(key) else {
            return 0;
        };
        let chosen = p.eval(self.eval).as_str();
        p.menu
            .as_ref()
            .and_then(|m| m.iter().position(|i| *i == chosen))
            .unwrap_or(0)
    }
    pub(crate) fn foreign(&self, i: usize) -> Option<&otd_core::Crossing> {
        self.foreign.get(i).and_then(|c| c.as_ref())
    }
    pub(crate) fn input(&self, i: usize) -> DatData {
        self.inputs.get(i).cloned().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Watched {
    pub values: IndexMap<String, f64>,
    pub started: bool,
}

pub struct DatSpec {
    pub def: OpDef,
    pub cook: fn(&mut DatCtx) -> DatData,
}

fn spec(
    type_name: &'static str,
    label: &'static str,
    inputs: &'static [&'static str],
    summary: &'static str,
    params: fn() -> IndexMap<String, Param>,
    cook: fn(&mut DatCtx) -> DatData,
) -> DatSpec {
    DatSpec {
        def: OpDef {
            type_name,
            label,
            family: Family::Dat,
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

fn specs() -> &'static Vec<DatSpec> {
    static SPECS: OnceLock<Vec<DatSpec>> = OnceLock::new();
    SPECS.get_or_init(table::core_specs)
}

pub fn spec_for(type_name: &str) -> Option<&'static DatSpec> {
    specs().iter().find(|s| s.def.type_name == type_name)
}

pub fn all() -> impl Iterator<Item = &'static DatSpec> {
    specs().iter()
}

pub fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    for s in specs() {
        r.register(s.def.clone());
    }
    r
}
