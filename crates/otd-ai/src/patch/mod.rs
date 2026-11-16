//! Turning a sentence into operators.

mod apply;
mod catalogue;
mod describe;
mod helpers;
mod json;
mod parse;
mod prompts;
mod shader_check;
mod types;

pub use types::{
    Applied, Plan, PlannedNode, PlannedSet, PlannedWire,
};
pub use catalogue::catalogue;
pub use describe::describe;
pub use prompts::{
    camera_prompt, reverse_engineer_prompt, result_prompt, system_prompt, NO_DELETE_RULE, SCOPE_RULE,
};
pub use json::extract_json;
pub use parse::{gate_scope, parse_plan};
pub use apply::{apply, splices_for};
pub use shader_check::{dangling, shader_problems, unresolved_refs, ShaderCheck};

#[cfg(test)]
#[path = "../tests/patch__tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/patch__shader_tests.rs"]
mod shader_tests;
