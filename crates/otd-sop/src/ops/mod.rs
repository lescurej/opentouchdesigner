//! The SOP operator table.
//!
//! Layout: `helpers`, `operators`, `table`.

use std::sync::OnceLock;

use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Graph, Node, OpDef, OpRegistry, Param, Value};

use crate::Geometry;

#[macro_use]
mod helpers;
mod table;
pub mod btog;

pub(crate) use helpers::try_locked_sidecar_geometry;

pub struct SopCtx<'a> {
    pub graph: &'a Graph,
    pub node: &'a Node,
    pub eval: &'a EvalContext<'a>,
    pub inputs: Vec<Geometry>,
}

impl SopCtx<'_> {
    fn val(&self, key: &str) -> Value {
        self.node
            .param(key)
            .map(|p| p.eval(self.eval))
            .unwrap_or(Value::Float(0.0))
    }
    fn f(&self, key: &str) -> f32 {
        self.val(key).as_f32()
    }
    fn i(&self, key: &str) -> i64 {
        self.val(key).as_i64()
    }
    fn b(&self, key: &str) -> bool {
        self.val(key).as_bool()
    }
    fn s(&self, key: &str) -> String {
        self.val(key).as_str()
    }
    fn v3(&self, key: &str) -> [f32; 3] {
        let v = self.val(key).as_vec4_f32();
        [v[0], v[1], v[2]]
    }
    fn v4(&self, key: &str) -> [f32; 4] {
        self.val(key).as_vec4_f32()
    }
    fn menu(&self, key: &str) -> usize {
        let Some(p) = self.node.param(key) else {
            return 0;
        };
        let chosen = p.eval(self.eval).as_str();
        p.menu_index(&chosen)
    }
    fn input(&self, i: usize) -> Geometry {
        self.inputs.get(i).cloned().unwrap_or_default()
    }
    fn resolve_file(&self, key: &str) -> std::path::PathBuf {
        self.graph.resolve_external(&self.s(key))
    }

    /// TouchDesigner `*.data` sidecar next to a SOP (e.g. expanded `.tox` tree).
    fn resolve_data_sidecar(&self) -> std::path::PathBuf {
        if let Some(path) = self.eval.path {
            let rel = path.trim_start_matches('/');
            if !rel.is_empty() {
                return self.graph.resolve_external(&format!("{rel}.data"));
            }
        }
        let name = format!("{}.data", self.node.name);
        self.graph.resolve_external(&name)
    }
}

pub struct SopSpec {
    pub def: OpDef,
    pub cook: fn(&mut SopCtx) -> Geometry,
}

include!(concat!(env!("OUT_DIR"), "/registry_generated.rs"));

pub const NULL: &str = "nullSOP";
pub const IN: &str = "inSOP";
pub const OUT: &str = "outSOP";

fn spec(
    type_name: &'static str,
    label: &'static str,
    inputs: &'static [&'static str],
    summary: &'static str,
    params: fn() -> IndexMap<String, Param>,
    cook: fn(&mut SopCtx) -> Geometry,
) -> SopSpec {
    SopSpec {
        def: OpDef {
            type_name,
            label,
            family: Family::Sop,
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

fn all_specs() -> Vec<SopSpec> {
    let mut v = folder_specs();
    v.extend(table::legacy_specs());
    v
}

fn specs() -> &'static Vec<SopSpec> {
    static SPECS: OnceLock<Vec<SopSpec>> = OnceLock::new();
    SPECS.get_or_init(all_specs)
}

pub fn spec_for(type_name: &str) -> Option<&'static SopSpec> {
    specs().iter().find(|s| s.def.type_name == type_name)
}

pub fn all() -> impl Iterator<Item = &'static SopSpec> {
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
