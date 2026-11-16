//! Placeholder 3D text geometry until full font extrusion lands.

use crate::{Geometry, Point, Topology};

/// Build viewer / render mesh for a Geo Text COMP from evaluated parameters.
pub fn cook_geometry(text: &str, scale: f32) -> Geometry {
    placeholder_geometry(text, scale)
}

/// A horizontal row of boxes, one per non-space character — enough to place and light 3D text.
pub fn placeholder_geometry(text: &str, scale: f32) -> Geometry {
    let scale = scale.max(0.01);
    let unit = scale * 0.02;
    let mut points = Vec::new();
    let mut indices = Vec::new();
    let mut x = 0.0f32;
    for ch in text.chars() {
        if ch.is_whitespace() {
            x += unit * 0.5;
            continue;
        }
        let hx = unit * 0.45;
        let hy = unit;
        let hz = unit * 0.15;
        let cx = x + hx;
        let base = points.len() as u32;
        let corners = [
            [-hx, -hy, -hz],
            [hx, -hy, -hz],
            [hx, hy, -hz],
            [-hx, hy, -hz],
            [-hx, -hy, hz],
            [hx, -hy, hz],
            [hx, hy, hz],
            [-hx, hy, hz],
        ];
        for c in corners {
            points.push(Point {
                position: [cx + c[0], c[1], c[2]],
                ..Point::default()
            });
        }
        for face in [
            [0, 1, 2, 3],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [0, 3, 7, 4],
            [1, 2, 6, 5],
        ] {
            indices.extend([base + face[0], base + face[1], base + face[2]]);
            indices.extend([base + face[0], base + face[2], base + face[3]]);
        }
        x += unit * 1.05;
    }
    if points.is_empty() {
        return Geometry::default();
    }
    Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    }
}
