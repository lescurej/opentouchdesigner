//! The graph model: an arena of nodes, typed by family, arranged in a
//! component hierarchy. No GPU, no UI, no Python — everything in here is
//! plain data so the cook engine can be unit tested (PLAN.md §3, "a clean
//! core").

mod edit;
mod model;
mod op_def;
mod paths;
mod resolve;
mod visit;

#[cfg(test)]
#[path = "../tests/graph_tests__tests.rs"]
mod tests;

pub use model::{Connector, Family, Graph, Node, NodeFlags, NodeId};
pub use op_def::{GraphError, OpDef, OpRegistry};
