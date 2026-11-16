//! `otd-core` — the graph model, the cook engine, and the project format.
//!
//! This crate deliberately has no GPU, windowing, UI or scripting dependency.
//! Everything here is plain data and scheduling logic, which is what makes the
//! cook engine unit-testable and keeps a future headless/WASM target open
//! (PLAN.md §3).

pub mod bundle;
pub mod callback;
pub mod channel_pattern;
pub mod clipboard;
pub mod component;
pub mod cook;
pub mod cook_session;
pub mod crossing;
pub mod edit;
pub mod expr;
pub mod graph;
pub mod shared_graph;
pub mod history;
pub mod io_async;
pub mod media;
pub mod movie_timecode;
pub mod missing;
pub mod osc;
pub mod param;
pub mod project;
pub mod python_runtime;
pub mod script;
pub mod tools;
pub mod value;

#[cfg(test)]
mod test_ops;

pub use bundle::Bundle;
pub use callback::{with_callback_network, with_network};
pub use channel_pattern::{
    ComponentPattern, NumberedPattern, channel_count_from_name, channel_name_at,
    channel_seed_for_name, component_pattern_matches, expand_channel_name_field,
    expand_channel_names, numbered_pattern_matches, operator_reference_is_pattern,
    parse_component_pattern, parse_numbered_pattern, pattern_matches_channel,
    pattern_matches_operator, rename_to_at,
};
pub use clipboard::{
    NodeClipboard, copy_selection, decode_operator_clipboard, encode_operator_clipboard, paste,
};
pub use component::Component;
pub use cook::{CookContext, CookEngine, CookError, CookOutput, Cooker, FrameStats};
pub use cook_session::{CookSession, cook_engine, in_cook_session, run_with_cook_session};
pub use crossing::{ChannelView, ChopTableMirror, Crossing, Crossings, MovieAudio, MoviePcm};
pub use edit::ParamEdit;
pub use expr::{ChannelSource, EvalContext, Expr};
pub use graph::{Connector, Family, Graph, GraphError, Node, NodeFlags, NodeId, OpDef, OpRegistry};
pub use shared_graph::SharedGraph;
pub use history::History;
pub use io_async::{cook_drain, drain_inbox, stop_io, trim_front};
pub use movie_timecode::movie_timecode_frame_index;
pub use missing::{
    MISSING_CHOP, MISSING_COMP, MISSING_DAT, MISSING_SOP, MISSING_TOP, ORIGINAL_TYPE_PARAM,
    ResolvedOpDef, display_op_type, finish_missing_substitution, is_missing_placeholder,
    missing_node_message, missing_params, missing_placeholder_type, node_entry_op_type,
    resolve_op_def,
};
pub use osc::{OscArg, encode_message};
pub use param::{AudioDeviceList, Param, ParamMode, SelectOption};
pub use project::{
    NetworkView, PROJECT_SAMPLE_RATE_DEFAULT, PROJECT_SAMPLE_RATES, Project, ProjectEditor,
    ProjectSettings, canonical_project_sample_rate, param_matches_default,
    project_sample_rate_label,
};
pub use script::{
    ScriptActions, ScriptQueue, WebSocketSend, flush_pending_sync_recooks, script_actions,
    take_queued_deferred_recooks, take_queued_edits, take_queued_udp_sends,
    take_queued_ws_sends_for,
};
pub use value::Value;

pub use indexmap;
