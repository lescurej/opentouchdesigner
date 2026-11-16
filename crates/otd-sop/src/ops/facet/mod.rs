
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point, Topology};

use super::helpers::normalise;
use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "mode" => Param::menu("smooth", &["smooth", "unique"]).with_label("Normals"),
    }
}

/// Recompute normals from the triangles. Every deformer here moves points
/// and leaves the normals where the generator put them, so a noised grid
/// lights like the flat grid it was; this is the fix. `unique` splits every
/// triangle into its own three points for a faceted look.
pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let g = c.input(0);
    if g.topology != Topology::Triangles {
        return g;
    }
    let idx: Vec<u32> = if g.indices.is_empty() {
        (0..g.points.len() as u32).collect()
    } else {
        g.indices.clone()
    };
    let face_normal = |t: &[u32]| {
        let [p0, p1, p2] = [0, 1, 2].map(|k| g.points[t[k] as usize].position);
        let (e1, e2) = (sub(p1, p0), sub(p2, p0));
        [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ]
    };
    if c.menu("mode") == 1 {
        let mut points = Vec::with_capacity(idx.len());
        for t in idx.chunks_exact(3) {
            let n = normalise(face_normal(t));
            points.extend(t.iter().map(|i| Point {
                normal: n,
                ..g.points[*i as usize]
            }));
        }
        return Geometry {
            points,
            indices: Vec::new(),
            topology: Topology::Triangles,
            ..Default::default()
        };
    }
    // Area-weighted: the unnormalised cross product is twice the area.
    let mut acc = vec![[0.0f32; 3]; g.points.len()];
    for t in idx.chunks_exact(3) {
        let n = face_normal(t);
        for i in t {
            for k in 0..3 {
                acc[*i as usize][k] += n[k];
            }
        }
    }
    g.map_points(|i, p| Point {
        normal: normalise(acc[i]),
        ..*p
    })
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub const TYPE_NAME: &str = "facetSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Facet",
        &["in"],
        "Recompute normals from the triangles.",
        params,
        cook,
    )
}
