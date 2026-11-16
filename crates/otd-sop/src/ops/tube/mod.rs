
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point, Topology};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "radius1" => Param::float(0.5).with_label("Bottom Radius").with_range(0.0, 8.0),
        "radius2" => Param::float(0.5).with_label("Top Radius").with_range(0.0, 8.0),
        "height" => Param::float(2.0).with_label("Height").with_range(0.0, 16.0),
        "columns" => Param::int(24).with_label("Columns").with_range(3.0, 256.0),
        "rows" => Param::int(1).with_label("Rows").with_range(1.0, 128.0),
        "caps" => Param::bool(true).with_label("Caps"),
        "center" => Param::xyz([0.0, 0.0, 0.0]).with_label("Center"),
    }
}

/// A tube, cone or cylinder depending on the two radii — one operator rather
/// than three, because they differ only in a number.
pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let (r1, r2) = (c.f("radius1"), c.f("radius2"));
    let h = c.f("height");
    let cols = c.i("columns").clamp(3, 256) as usize;
    let rows = c.i("rows").clamp(1, 128) as usize;
    let o = c.v3("center");

    let mut points = Vec::new();
    for row in 0..=rows {
        let v = row as f32 / rows as f32;
        let y = o[1] - h * 0.5 + h * v;
        let r = r1 + (r2 - r1) * v;
        for col in 0..=cols {
            let u = col as f32 / cols as f32;
            let a = u * std::f32::consts::TAU;
            let (s, cs) = (a.sin(), a.cos());
            // The side normal leans by the taper, so a cone shades as a cone
            // rather than as a cylinder that happens to be narrower at one
            // end.
            let slope = (r1 - r2) / h.max(1e-5);
            let n = normalise([cs, slope, s]);
            points.push(Point {
                position: [o[0] + cs * r, y, o[2] + s * r],
                normal: n,
                uv: [u, v],
                color: [1.0; 4],
            });
        }
    }

    let stride = cols + 1;
    let mut indices = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let a = (row * stride + col) as u32;
            let b = a + stride as u32;
            // The opposite order to the Sphere's: its rows run top to bottom
            // and these run bottom to top, so the same index pattern would
            // wind the sides inside out.
            indices.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }

    if c.b("caps") {
        for (end, r, ny) in [(0.0f32, r1, -1.0f32), (1.0, r2, 1.0)] {
            if r <= 0.0 {
                continue; // A cone's point needs no disc.
            }
            let y = o[1] - h * 0.5 + h * end;
            let centre = points.len() as u32;
            points.push(Point {
                position: [o[0], y, o[2]],
                normal: [0.0, ny, 0.0],
                uv: [0.5, 0.5],
                color: [1.0; 4],
            });
            for col in 0..=cols {
                let a = col as f32 / cols as f32 * std::f32::consts::TAU;
                points.push(Point {
                    position: [o[0] + a.cos() * r, y, o[2] + a.sin() * r],
                    normal: [0.0, ny, 0.0],
                    uv: [a.cos() * 0.5 + 0.5, a.sin() * 0.5 + 0.5],
                    color: [1.0; 4],
                });
            }
            for col in 0..cols {
                let a = centre + 1 + col as u32;
                // The two caps wind opposite ways, since they face opposite
                // ways and culling is on. The ring runs anticlockwise in XZ,
                // so taking it in order gives a face normal of -Y.
                if ny < 0.0 {
                    indices.extend([centre, a, a + 1]);
                } else {
                    indices.extend([centre, a + 1, a]);
                }
            }
        }
    }

    Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    }
}

fn normalise(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / len, v[1] / len, v[2] / len]
}
pub const TYPE_NAME: &str = "tubeSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Tube",
        &[],
        "A cylinder, cone or tapered tube, with optional caps.",
        params,
        cook,
    )
}
