//! The TOP operator table.
//!
//! Adding an operator is: one `.wgsl` file, one `params_*` function, one
//! `pack_*` function, one `TopSpec` entry. That ratio is deliberate — PLAN.md
//! §6 names operator breadth as the main treadmill risk, so the per-operator
//! cost has to stay near zero.
//!
//! Layout:
//! - `helpers` — parameter evaluation helpers
//! - `definitions` — `params_*` / `pack_*` for each operator
//! - `blend` — named blend operators (shared composite shader)
//! - `macros` — `filter_top!` / `blend_top!`
//! - `extra` — TD-style filter TOPs registered via macros
//! - `table` — core TOP table entries

use std::sync::OnceLock;

use otd_core::{Connector, EvalContext, Family, Node, OpDef, OpRegistry};

#[macro_use]
mod helpers;
#[macro_use]
mod macros;
mod definitions;
mod blend;
mod extra;
mod table;

pub use definitions::{
    CACHE, CHOP_TO_TOP, FEEDBACK, GLSL, IN, MOVIE_IN, MOVIE_OUT, NULL, OUT, RENDER, SELECT, TEXT,
    VIDEO_DEVICE_IN, movie_out_length_limit_frames, nothing_to_open, shader_source,
};
pub use helpers::{generator_size, movie_fit_mode, movie_output_dimensions, movie_sampler_nearest};

pub const COMMON_WGSL: &str = include_str!("../shaders/common.wgsl");

pub type PackedParams = [[f32; 4]; 4];

pub const PARAM_VECS: usize = 12;

pub type WideParams = [[f32; 4]; PARAM_VECS];

pub const NO_PARAMS: WideParams = [[0.0; 4]; PARAM_VECS];

pub fn widen(packed: PackedParams) -> WideParams {
    let mut out = NO_PARAMS;
    out[..4].copy_from_slice(&packed);
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sizing {
    Params,
    Input0,
    Input0OrParams,
    Referenced,
}

pub struct TopSpec {
    pub def: OpDef,
    pub shader: &'static str,
    pub sizing: Sizing,
    pub two_pass: bool,
    pub dynamic_shader: bool,
    pub pack: fn(&Node, &EvalContext) -> PackedParams,
}

impl TopSpec {
    pub fn pack_params(&self, node: &Node, ctx: &EvalContext) -> WideParams {
        if self.def.type_name == GLSL {
            definitions::pack_glsl_wide(node, ctx)
        } else {
            widen((self.pack)(node, ctx))
        }
    }
}

fn specs() -> &'static Vec<TopSpec> {
    static SPECS: OnceLock<Vec<TopSpec>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let mut v = table::core_specs();
        v.extend(extra::extra_specs());
        v
    })
}

pub fn spec(type_name: &str) -> Option<&'static TopSpec> {
    specs().iter().find(|s| s.def.type_name == type_name)
}

pub fn all() -> impl Iterator<Item = &'static TopSpec> {
    specs().iter()
}

pub const WINDOW: &str = "windowCOMP";

pub const WINDOW_COMP: OpDef = OpDef {
    type_name: WINDOW,
    input_families: &[],
    label: "Window",
    family: Family::Comp,
    inputs: &[],
    summary: "Floating or external display for a cooked TOP (TouchDesigner Window COMP).",
    time_dependent: false,
    params: definitions::params_window_comp,
    connector: Connector::None,
};

/// Every operator this build knows about.
pub fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    for s in specs() {
        r.register(s.def.clone());
    }
    r.register(WINDOW_COMP);
    r
}

#[cfg(test)]
mod video_source_tests {
    use super::nothing_to_open;

    #[test]
    fn a_blank_device_means_the_default_camera_not_nothing() {
        // The regression this pins: a Video Device In is created with `device`
        // blank, because its label says blank is the default. Reading that as
        // "no source" made the node the user had just added the one node that
        // could never produce a picture — black, silent, and with no ffmpeg
        // started, so nothing downstream of it and nothing in any log said why.
        assert!(!nothing_to_open(true, ""), "blank device = default camera");
        assert!(!nothing_to_open(true, "   "));
        assert!(!nothing_to_open(true, "1"), "a named device, obviously");

        // A movie with no file really is nothing to open.
        assert!(nothing_to_open(false, ""));
        assert!(nothing_to_open(false, "  \t "));
        assert!(!nothing_to_open(false, "/clips/a.mov"));
    }
}

#[cfg(test)]
mod blend_alias_tests {
    use super::*;
    use crate::ops::definitions::params_composite;

    /// Every named blend operator has to pin the Composite TOP operation of
    /// the same name.
    ///
    /// They all share `composite.wgsl` and differ only in the index their
    /// `pack` puts in `p0.x`, so a transposed pair here is a Subtract TOP that
    /// silently screens — no crash, no failing shader, just a wrong picture.
    /// This reads the index back out of the Composite TOP's own menu, so the
    /// menu and the aliases cannot drift apart either.
    #[test]
    fn each_named_blend_pins_the_composite_operation_it_is_named_after() {
        let menu = params_composite()["operation"].menu.clone().unwrap();
        let ctx = EvalContext::default();
        let registry = registry();
        let mut graph = otd_core::Graph::new();
        let root = graph.root();

        for (type_name, operation) in [
            ("overTOP", "over"),
            ("underTOP", "under"),
            ("addTOP", "add"),
            ("subtractTOP", "subtract"),
            ("multiplyTOP", "multiply"),
            ("screenTOP", "screen"),
            ("differenceTOP", "difference"),
            ("insideTOP", "inside"),
            ("outsideTOP", "outside"),
            ("crossTOP", "cross"),
        ] {
            let spec = spec(type_name).expect("registered");
            let id = graph
                .create(root, registry.get(type_name).unwrap(), None)
                .unwrap();
            let node = graph.node(id);
            let expected = menu.iter().position(|m| m == operation).expect("in menu");
            assert_eq!(
                (spec.pack)(node, &ctx)[0][0] as usize,
                expected,
                "{type_name} should composite with `{operation}`"
            );
        }
    }
}
