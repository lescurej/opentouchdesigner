//! The 3D scene: Geometry, Camera and Light components, materials, and the
//! instance data that feeds the renderer.
//!
//! PLAN.md Phase 4 names "instancing from CHOP/DAT/TOP sources (texture-based
//! instancing = the signature TD trick)" as the thing to get right. Both
//! paths are here and both end in the same place — a per-instance transform
//! the vertex shader applies:
//!
//!  * **From a CHOP**: each *sample* is an instance. A Pattern CHOP of 2000
//!    samples is 2000 instances, and the channels are named by parameter.
//!  * **From a TOP**: the texture is read in the vertex shader by instance
//!    index. Nothing comes back to the CPU, which is the whole point — a
//!    feedback loop can compute a million positions and none of them cross
//!    the bus.

use otd_sop::Geometry;

/// What the renderer needs from the rest of the network.
///
/// Implemented by the cross-family engine; `otd-gpu` never looks up a path
/// itself.
pub trait Scene {
    /// The geometry a SOP (or a component wrapping one) is presenting.
    fn geometry(&self, path: &str) -> Option<&Geometry>;
    /// A CHOP's channels, by name, with all their samples.
    fn channels(&self, path: &str) -> Option<Vec<(String, Vec<f32>)>>;
}

mod helpers;
mod instance;
mod params;
mod defs;

pub use helpers::{b, f, menu, s, v3, v4};
pub use instance::{instances, Instance};
pub use defs::{register_all, defs, Shading, CAMERA, CONSTANT_MAT, GEOMETRY, LIGHT, PBR, PHONG, POINT_SPRITE, WIREFRAME};
