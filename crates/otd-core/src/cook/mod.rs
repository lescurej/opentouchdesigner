//! The cook engine: demand-driven, pull-based, lazily memoized.
//!
//! PLAN.md §2.2 / §4. The rules, in one place:
//!
//!  * Cooking is **pulled from roots**, never pushed from edits. The app uses
//!    every operator as a root so nothing stops when it leaves the canvas.
//!  * A node cooks iff (a) it changed since it last cooked, (b) one of its
//!    inputs produced a new output, or (c) it is **time dependent**.
//!  * Time dependence propagates downstream: a node fed by an animated node is
//!    itself animated. A branch with neither an animated parameter nor an
//!    animated input cooks once and then returns its cached output forever.
//!    This is the property that makes a big patch cheap.
//!  * Each node resolves at most once per `cook_frame` call, so diamonds cook
//!    their shared ancestor once. The token is a pass counter, not the
//!    timeline frame: two repaints can share a frame number (a 120 Hz display
//!    with a 60 fps project), and time-sliced audio still has to cook for each.
//!
//! The engine owns *scheduling only*. Producing the actual output — a texture,
//! a channel buffer — is the [`Cooker`]'s job, which is how `otd-core` stays
//! free of any GPU dependency.


mod context;
mod engine;
mod types;

pub use context::CookContext;
pub use engine::CookEngine;
pub use types::{CookError, CookOutput, Cooker, FrameStats};

use crate::graph::{Graph, NodeId};

/// The node that actually produced the output `id` presents, following
/// bypass flags and stepping out of components.
///
/// Kept as a free function for the engines that were written against it; the
/// logic lives on [`Graph`].
pub fn resolve_bypass(graph: &Graph, id: NodeId) -> Option<NodeId> {
    graph.resolve_output(id)
}

#[cfg(test)]
#[path = "../tests/cook__tests.rs"]
mod tests;
