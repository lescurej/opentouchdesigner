//! User shader handling for the GLSL TOP.
//!
//! PLAN.md §3: "WGSL primary, GLSL accepted — paste Shadertoy GLSL and it
//! works." Two things make that practical:
//!
//!  * **Wrapping.** The artist writes a fragment *body* in WGSL, or a
//!    Shadertoy `mainImage` in GLSL. The boilerplate — bindings, the
//!    fullscreen vertex stage, the `iTime`/`iResolution` names — is supplied.
//!    A source that already declares its own entry point is passed through
//!    untouched, so nothing is locked out.
//!  * **Validating before wgpu sees it.** naga is run over the source first.
//!    A typo then produces a message with a line number instead of a device
//!    error, which is the difference between a usable live-coding surface and
//!    an unusable one.


mod wrap;
mod hoist;
mod validate;

pub use wrap::{adopt_td_uniforms, wrap_glsl, wrap_glsl_full, wrap_wgsl, VERTEX_GLSL, VERTEX_ONLY_WGSL};
pub use validate::{validate_glsl, validate_wgsl};

#[cfg(test)]
pub(crate) use wrap::flip_channel_lookups;

#[cfg(test)]
#[path = "../tests/shader__channel_flip.rs"]
mod channel_flip;
