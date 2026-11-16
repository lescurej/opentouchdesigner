//! The Render TOP's pipeline: depth buffer, instanced draw, one light.
//!
//! Kept apart from the 2D `TopEngine` because it is a genuinely different
//! pipeline — vertex buffers, a depth attachment, a cull mode — and mixing
//! the two would make both harder to read. The engine owns one of these and
//! calls it for `renderTOP` nodes.

mod types;
mod preview;
mod scene;
mod material;
mod renderer;

pub use types::{DrawItem, SceneDescription, DEPTH_FORMAT};
pub use preview::{describe_sop_preview, sop_preview_cache_key, OrbitCamera};
#[cfg(test)]
pub(crate) use preview::fit_distance;
pub use scene::describe;
pub use material::Surface;
pub use renderer::{depth_texture, Renderer};

#[cfg(test)]
#[path = "../tests/render3d__tests.rs"]
mod tests;
