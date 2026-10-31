use otd_core::indexmap::IndexMap;
use otd_core::{EvalContext, Node, Param};

use super::helpers::*;
use super::PackedParams;

// The named blend operators. Each is a Composite TOP with its Operation fixed,
// which is what makes them one line each: the alternative, an operator apiece
// with its own shader, is eleven copies of the same twenty lines.
pub(crate) fn params_blend() -> IndexMap<String, Param> {
    params! {
        "opacity" => Param::float(1.0).with_label("Opacity").with_range(0.0, 1.0),
    }
}

macro_rules! blend_pack {
    ($name:ident, $op:expr) => {
        pub(crate) fn $name(n: &Node, c: &EvalContext) -> PackedParams {
            [
                [$op as f32, f(n, c, "opacity"), 0.0, 0.0],
                [0.0; 4],
                [0.0; 4],
                [0.0; 4],
            ]
        }
    };
}

blend_pack!(pack_over, 0);
blend_pack!(pack_add, 1);
blend_pack!(pack_multiply, 2);
blend_pack!(pack_screen, 3);
blend_pack!(pack_difference, 4);
blend_pack!(pack_subtract, 5);
blend_pack!(pack_under, 8);
blend_pack!(pack_inside, 9);
blend_pack!(pack_outside, 10);
blend_pack!(pack_cross, 11);
