
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::Geometry;

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "blend" => Param::float(0.0).with_label("Blend").with_range(0.0, 1.0),
        "match" => Param::menu("stretch", &["stretch", "index"]).with_label("Match Points"),
        "attributes" => Param::bool(true).with_label("Blend Normals and Color"),
    }
}

/// Morph between two pieces of geometry.
///
/// Interpolating point positions is the whole trick, and it only works when
/// the two shapes agree about which point is which. They almost never do — a
/// torus has 1024 points and a sphere has 561 — so the interesting parameter
/// is `match`, which is how the correspondence is invented:
///
///  * **stretch** walks input B proportionally. Point 0 of a 100-point shape
///    pairs with point 0 of a 500-point one, point 50 with point 250. Both
///    surfaces are traversed end to end, so a morph between two different
///    primitives moves every point rather than leaving most of one shape
///    stationary. This is the one you want, which is why it is the default.
///  * **index** pairs point *n* with point *n* and stops at the shorter of
///    the two. Right when the shapes are the same topology deformed two ways
///    — a grid and a displaced copy of it — where stretch would slide points
///    along the surface and shear the result.
///
/// The output keeps input A's topology and indices. That is a deliberate
/// asymmetry: blend is a *deformation of A towards B*, so the thing being
/// drawn stays the thing you wired into the first input, and at blend 1 you
/// have A's connectivity holding B's shape. Producing a shape whose triangles
/// rewire themselves halfway through would not be a morph, it would be a cut.
pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let a = c.input(0);
    let b = c.input(1);
    if a.is_empty() {
        return b;
    }
    if b.is_empty() {
        return a;
    }

    let t = c.f("blend").clamp(0.0, 1.0);
    let stretch = c.menu("match") == 0;
    let attributes = c.b("attributes");

    let mut out = a;
    let n_b = b.points.len();
    let n_a = out.points.len();

    for (i, p) in out.points.iter_mut().enumerate() {
        let j = if stretch && n_a > 1 {
            // Proportional position along B, rounded to a real point.
            let f = i as f64 / (n_a - 1) as f64;
            ((f * (n_b - 1) as f64).round() as usize).min(n_b - 1)
        } else {
            i.min(n_b - 1)
        };
        // `index` matching past the end of B holds the last point rather than
        // collapsing to the origin, which is what a bounds-check returning a
        // default would do and looks like the model exploded.
        let q = &b.points[j];
        for k in 0..3 {
            p.position[k] += (q.position[k] - p.position[k]) * t;
        }
        if attributes {
            for k in 0..3 {
                p.normal[k] += (q.normal[k] - p.normal[k]) * t;
            }
            for k in 0..4 {
                p.color[k] += (q.color[k] - p.color[k]) * t;
            }
            // Interpolated normals stop being unit length; renormalise or the
            // lighting dims through the middle of the morph.
            let n = &mut p.normal;
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 1e-6 {
                n[0] /= len;
                n[1] /= len;
                n[2] /= len;
            }
        }
    }
    out
}
pub const TYPE_NAME: &str = "blendSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Blend",
        &["a", "b"],
        "Morph between two shapes by interpolating point positions.",
        params,
        cook,
    )
}
