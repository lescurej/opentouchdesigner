//! The CHOP operator table.
//!
//! Same shape as the TOP table in `otd-gpu`: one cook function and one entry
//! per operator, so breadth stays cheap (PLAN.md §6).
//!
//! Layout: `helpers`, `ops/<slug>/mod.rs` (build.rs registry), `table` (rt marks).
//! Device I/O operators live in `crate::io`; `ops/<slug>/mod.rs` wires `spec()` to `crate::io`.

use std::sync::{Arc, OnceLock};

use otd_core::indexmap::IndexMap;
use otd_core::{
    Connector, CookContext, Crossings, EvalContext, Family, Node, OpDef, OpRegistry, Param, Value,
};

#[macro_use]
mod helpers;
#[allow(dead_code)]
mod resample_elemental;
mod shared;
mod table;

include!(concat!(env!("OUT_DIR"), "/registry_generated.rs"));

pub(crate) use filein::cook_file_in_shared;

pub use constant::{CONSTANT, CONSTANT_MAX_ROWS};
pub use info::INFO;
pub(crate) use lfo::wave;
pub(crate) use null::cook as cook_null;
pub use parameter::PARAMETER;
pub use perform::PERFORM;

use crate::data::ChopData;
use crate::engine::ChannelStore;
use helpers::no_params;

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
    pub inputs: Vec<Arc<crate::data::ChopData>>,
    pub foreign: Crossings,
    pub state: &'a mut OpState,
    pub io: &'a mut crate::io::Io,
    pub path: &'a str,
    pub store: &'a ChannelStore,
    /// Set by the audio worker: every CHOP in a compiled audio plan runs at
    /// the device rate, whatever its own Sample Rate parameter says.
    pub rate_override: Option<f64>,
    /// Target channels captured at the end of the previous frame.
    pub prev_frame: Option<Arc<ChopData>>,
    /// Feedback writes the node whose output should be held for the next frame.
    pub capture: Option<otd_core::NodeId>,
    /// Feedback sets this when Reset should drop the held channels.
    pub clear_prev: bool,
    /// Reuse input `inputs[i]` as output without copying sample buffers.
    pub passthrough_input: Option<usize>,
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
        param.menu_index(&chosen)
    }
    pub fn foreign(&self, index: usize) -> Option<&otd_core::Crossing> {
        self.foreign.get(index).and_then(|c| c.as_ref())
    }
    pub fn input(&self, index: usize) -> &crate::data::ChopData {
        static EMPTY: OnceLock<Arc<crate::data::ChopData>> = OnceLock::new();
        self.inputs
            .get(index)
            .map(|a| a.as_ref())
            .unwrap_or_else(|| {
                EMPTY
                    .get_or_init(|| Arc::new(crate::data::ChopData::empty()))
                    .as_ref()
            })
    }
    pub fn rate(&self) -> f64 {
        if let Some(r) = self.rate_override {
            return r;
        }
        let r = self.f("rate") as f64;
        if r > 0.0 {
            r
        } else {
            crate::data::CONTROL_RATE
        }
    }
}

pub struct ChopSpec {
    pub def: OpDef,
    pub cook: fn(&mut ChopCtx) -> crate::data::ChopData,
    pub rt_safe: bool,
    /// A terminal the audio worker drives: Audio Device Out or Reaper Out.
    pub rt_audio_sink: bool,
    pub rt_audio_sink_priority: u8,
}

/// `Audio Device Out` — owns the device clock; one per plan.
pub const RT_AUDIO_SINK_DEVICE: u8 = 0;
/// `Reaper Out` — streams on the worker clock; any number per plan.
pub const RT_AUDIO_SINK_REAPER: u8 = 1;

pub(crate) fn mark_rt_audio_sink(mut s: ChopSpec, priority: u8) -> ChopSpec {
    s.rt_audio_sink = true;
    s.rt_audio_sink_priority = priority;
    s
}

pub const NULL: &str = "nullCHOP";
pub use connector_in::TYPE_NAME as IN;
pub use out::TYPE_NAME as OUT;

fn all_specs_vec() -> Vec<ChopSpec> {
    let mut v = folder_specs();
    table::mark_rt_safe_chops(&mut v);
    v
}

fn specs() -> &'static Vec<ChopSpec> {
    static SPECS: OnceLock<Vec<ChopSpec>> = OnceLock::new();
    SPECS.get_or_init(all_specs_vec)
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
        rt_safe: false,
        rt_audio_sink: false,
        rt_audio_sink_priority: u8::MAX,
    }
}

pub(crate) fn spec_animated_rt(
    type_name: &'static str,
    label: &'static str,
    inputs: &'static [&'static str],
    summary: &'static str,
    params: fn() -> IndexMap<String, Param>,
    cook: fn(&mut ChopCtx) -> ChopData,
) -> ChopSpec {
    let mut s = spec_animated(type_name, label, inputs, summary, params, cook);
    s.rt_safe = true;
    s
}

pub fn is_rt_safe_chop(type_name: &str) -> bool {
    spec_for(type_name).is_some_and(|s| s.rt_safe)
}

pub fn is_rt_audio_sink(type_name: &str) -> bool {
    spec_for(type_name).is_some_and(|s| s.rt_audio_sink)
}

pub fn is_rt_device_sink(type_name: &str) -> bool {
    spec_for(type_name)
        .is_some_and(|s| s.rt_audio_sink && s.rt_audio_sink_priority == RT_AUDIO_SINK_DEVICE)
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

/// Cross-family source is named on a parameter (TouchDesigner-style), not wired.
pub(crate) fn converter_spec(
    type_name: &'static str,
    label: &'static str,
    summary: &'static str,
    params: fn() -> IndexMap<String, Param>,
    cook: fn(&mut ChopCtx) -> ChopData,
) -> ChopSpec {
    spec(type_name, label, &[], summary, params, cook)
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
