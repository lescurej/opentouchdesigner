//! `otd-core` — the graph model, the cook engine, and the project format.
//!
//! This crate deliberately has no GPU, windowing, UI or scripting dependency.
//! Everything here is plain data and scheduling logic, which is what makes the
//! cook engine unit-testable and keeps a future headless/WASM target open
//! (PLAN.md §3).

pub mod bundle;
pub mod channel_pattern;
pub mod clipboard;
pub mod component;
pub mod cook;
pub mod crossing;
pub mod callback;
pub mod osc;
pub mod edit;
pub mod script;
pub mod expr;
pub mod graph;
pub mod history;
pub mod media;
pub mod param;
pub mod project;
pub mod python_runtime;
pub mod tools;
pub mod value;

#[cfg(test)]
mod test_ops;

pub use bundle::Bundle;
pub use channel_pattern::{
    channel_count_from_name, channel_name_at, channel_seed_for_name, component_pattern_matches,
    expand_channel_name_field, expand_channel_names, numbered_pattern_matches,
    parse_component_pattern, parse_numbered_pattern, pattern_matches_channel, rename_to_at,
    ComponentPattern, NumberedPattern,
};
pub use clipboard::{
    copy_selection, decode_operator_clipboard, encode_operator_clipboard, paste, NodeClipboard,
};
pub use component::Component;
pub use cook::{CookContext, CookEngine, CookError, CookOutput, Cooker, FrameStats};
pub use crossing::{ChannelView, Crossing, Crossings};
pub use callback::{CallbackGuard, with_network};
pub use osc::{OscArg, encode_message};
pub use edit::ParamEdit;
pub use script::{
    ScriptActions, ScriptQueue, WebSocketSend, flush_pending_sync_recooks, script_actions,
    set_sync_cook_flush, take_queued_deferred_recooks, take_queued_edits, take_queued_udp_sends,
    take_queued_ws_sends_for,
};
pub use expr::{ChannelSource, EvalContext, Expr};
pub use graph::{Connector, Family, Graph, GraphError, Node, NodeFlags, NodeId, OpDef, OpRegistry};
pub use history::History;
pub use param::{Param, ParamMode};
pub use project::{Project, ProjectSettings};
pub use value::Value;

pub use indexmap;
