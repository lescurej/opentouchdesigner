//! The DAT operator table.
//!
//! Layout: `helpers`, `ops/<slug>/mod.rs` (build.rs registry), `constants`.

use std::sync::OnceLock;

use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Crossings, EvalContext, Family, Node, OpDef, OpRegistry, Param, Value};

use crate::{DatData, ScriptHost};

#[macro_use]
mod helpers;
mod callbacks;
mod constants;
mod shared;
include!(concat!(env!("OUT_DIR"), "/registry_generated.rs"));

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
    pub input_keys: Option<&'a [String]>,
    /// Resolved CHOP for `choptodatDAT`: absolute path and dimensions.
    pub chop_table_src: Option<(String, usize, usize)>,
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
        p.menu_index(&chosen)
    }
    pub(crate) fn input(&self, i: usize) -> DatData {
        self.inputs.get(i).cloned().unwrap_or_default()
    }
    pub(crate) fn foreign(&self, i: usize) -> Option<&otd_core::Crossing> {
        self.foreign.get(i).and_then(|c| c.as_ref())
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

pub(crate) fn spec(
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
    SPECS.get_or_init(folder_specs)
}

pub fn spec_for(type_name: &str) -> Option<&'static DatSpec> {
    specs().iter().find(|s| s.def.type_name == type_name)
}

pub fn all() -> impl Iterator<Item = &'static DatSpec> {
    specs().iter()
}

pub fn register_all(registry: &mut OpRegistry) {
    for s in specs() {
        registry.register(s.def.clone());
    }
}

pub fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    register_all(&mut r);
    r
}
