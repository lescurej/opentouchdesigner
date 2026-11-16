//! The text project format.
//!
//! PLAN.md calls a git-diffable project format a headline feature, so it is
//! designed here rather than retrofitted. Two decisions carry that:
//!
//!  * **Nodes are a flat, path-sorted list**, not a nested tree. Adding a node
//!    to a component appends one block instead of re-indenting a subtree.
//!  * **Everything defaultable is omitted.** A freshly created operator writes
//!    only its path, type and position; parameters appear in the file only
//!    once the artist has touched them. Diffs then read as a list of the
//!    actual authoring decisions.
//!
//! Wiring is stored on the *consumer* as input paths, which keeps a rewire to
//! a single changed line.
//!
//! The editor camera is not part of the graph. It is an `editor` block —
//! which network was open, and the pan and zoom of each network — and it is
//! left out while every view is still the default.


mod io;
mod types;

pub use types::{
    DEFAULT_MAX_TIME_SLICE_MS, DEFAULT_PROJECT_FPS, FORMAT_ID, FORMAT_VERSION, NetworkView,
    NodeEntry, Project, ProjectEditor, ProjectError, ProjectSettings, PROJECT_SAMPLE_RATE_DEFAULT,
    PROJECT_SAMPLE_RATES, canonical_project_sample_rate, param_matches_default,
    project_sample_rate_label,
};
pub(crate) use io::{node_entry, restore_node};
