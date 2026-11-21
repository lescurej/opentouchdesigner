//! `otd-engine` — the cross-family cook.
//!
//! `otd-core` drives scheduling; `otd-gpu` and `otd-chop` implement one family
//! each and do not depend on each other (`otd-gpu` uses `otd-core` for shared
//! movie timecode indexing). This crate wires families into one cook, docs,
//! and runtime so a TOP parameter bound to a CHOP via `ch()` reads
//! channels from the same frame (PLAN.md §2.3).
//!
//! The channel store is owned here rather than by the CHOP engine so that,
//! while a node is cooking, the freshly cooked channels can be read
//! immutably by the parameters of whatever cooks next.

pub mod audio_partition;
mod audio_runtime;
pub mod comp;
pub mod container;
mod cook_roots;
pub use audio_partition::{
    AudioPartition, graph_audio_generation, node_in_audio_domain, partition_audio_domain,
    skip_frame_chop_cook,
};
pub use audio_runtime::AudioRuntime;
mod background_cook;
pub mod demo;
pub mod docs;
pub use background_cook::{CookHint, CookReport, background_cook_loop};
pub mod engine_client;
pub mod execute;
pub mod geotext_comp;
mod info;
pub mod lint;
mod node_message;
pub mod palette;
pub use otd_panel as panel;
pub use otd_panel::layout as panel_layout;
pub use otd_panel::{params as panel_params, templates as panel_templates};
pub mod project_runtime;
mod registry;
pub mod replicator;
pub mod text_comp;

pub use container::{
    CONTAINER, Kind, Widget, container_view_pixel_size, is_container_type,
    is_panel_child_base_comp, panel_display_widgets, widgets,
};

pub use cook_roots::{
    CookRootsRequest, EditorRootsRequest, OutputRootsRequest, app_cook_roots, editor_cook_roots,
    editor_roots, is_side_effect_op, operators_watched_by_info_parameter, output_roots,
};
pub use engine_client::{EngineClient, FrameEvent, TickHint};

pub fn is_side_effect_root(op: &str) -> bool {
    is_side_effect_op(op)
}
pub use info::node_infotip;
pub use node_message::{NodeMessage, NodeMessageLevel, classify_node_message};
pub use otd_py::{LintSeverity, PythonLint};
pub use project_runtime::{
    CallbackFrameStats, EguiDisplayBlit, FrameConfig, FrameResult, FrameTiming, GraphSyncStats,
    PlaybackState, ProjectRuntime, clip_frame_dt, summarize, tick_frame_state,
};

mod cooker;
mod engines;
mod network;
mod scene_view;
mod script_host;

pub use comp::{network_output_port_count, network_output_port_family};
pub use engines::Engines;
pub use network::dat_data_at;
pub use otd_chop::engine::ChannelStore;
pub use otd_chop::{Channel, ChopData};
pub use otd_dat::DatData;
pub use otd_dat::DatStore;
pub use otd_sop::Geometry as SopGeometry;
pub use otd_sop::GeometryStore;
pub use registry::{register_all, registry};
