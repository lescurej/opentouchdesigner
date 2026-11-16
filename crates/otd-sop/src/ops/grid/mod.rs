
use otd_core::indexmap::IndexMap;
use otd_core::{Param, Value};

use crate::{Geometry, Point, Topology};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "size" => Param::new(Value::Vec2([2.0, 2.0])).with_label("Size"),
        "rows" => Param::int(10).with_label("Rows").with_range(2.0, 512.0),
        // TouchDesigner names this `cols`. `columns` stays so existing patches keep their value.
        "cols" => Param::int(10).with_label("Cols").with_range(2.0, 512.0),
        "columns" => Param::int(10).with_label("Columns").with_range(2.0, 512.0),
        "orientation" => Param::menu("xy", &["xy", "xz", "yz"]).with_label("Orientation"),
    }
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let s = c.v4("size");
    let rows = c.i("rows").clamp(2, 512) as usize;
    let cols = grid_columns(c);
    let orient = c.menu("orientation");

    let place = |a: f32, b: f32| -> ([f32; 3], [f32; 3]) {
        match orient {
            1 => ([a, 0.0, b], [0.0, 1.0, 0.0]),
            2 => ([0.0, a, b], [1.0, 0.0, 0.0]),
            _ => ([a, b, 0.0], [0.0, 0.0, 1.0]),
        }
    };

    let mut points = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        let v = row as f32 / (rows - 1) as f32;
        for col in 0..cols {
            let u = col as f32 / (cols - 1) as f32;
            let (position, normal) = place((u - 0.5) * s[0], (v - 0.5) * s[1]);
            points.push(Point {
                position,
                normal,
                uv: [u, v],
                color: [1.0; 4],
            });
        }
    }

    let mut indices = Vec::with_capacity((rows - 1) * (cols - 1) * 6);
    for row in 0..rows - 1 {
        for col in 0..cols - 1 {
            let a = (row * cols + col) as u32;
            let b = a + cols as u32;
            // Counter-clockwise seen from the side the normals face.
            indices.extend([a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    }
}

/// `cols` is the TouchDesigner name. A patch that only set `columns` keeps that
/// count while `cols` is still the default.
fn grid_columns(c: &SopCtx) -> usize {
    let cols = c.i("cols");
    let columns = c.i("columns");
    let n = if cols != 10 { cols } else { columns };
    n.clamp(2, 512) as usize
}
pub const TYPE_NAME: &str = "gridSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Grid",
        &[],
        "A flat grid of quads — the usual thing to displace.",
        params,
        cook,
    )
}
