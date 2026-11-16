//! TouchDesigner `.gnode` — stored component transform (UT_DMatrix4 rows).

use otd_core::graph::{Graph, NodeId};
use otd_core::value::Value;

/// Apply transform hints from a `.gnode` file onto a component's parameters.
pub fn apply_gnode_transform(graph: &mut Graph, id: NodeId, text: &str) {
    let Some(matrix) = parse_primary_matrix(text) else {
        return;
    };
    let (tx, ty, tz) = matrix.translation();
    let (rx, ry, rz) = matrix.euler_xyz_degrees();
    let scale = matrix.uniform_scale();

    for (key, val) in [
        ("tx", Value::Float(tx as f64)),
        ("ty", Value::Float(ty as f64)),
        ("tz", Value::Float(tz as f64)),
        ("rx", Value::Float(rx as f64)),
        ("ry", Value::Float(ry as f64)),
        ("rz", Value::Float(rz as f64)),
        ("scale", Value::Float(scale as f64)),
        ("translatex", Value::Float(tx as f64)),
        ("translatey", Value::Float(ty as f64)),
        ("translatez", Value::Float(tz as f64)),
    ] {
        if graph.node(id).params.contains_key(key) {
            let _ = graph.set_param(id, key, val);
        }
    }
}

struct Matrix4 {
    m: [[f32; 4]; 4],
}

impl Matrix4 {
    fn translation(&self) -> (f32, f32, f32) {
        (self.m[3][0], self.m[3][1], self.m[3][2])
    }

    fn uniform_scale(&self) -> f32 {
        let sx = (self.m[0][0].powi(2) + self.m[0][1].powi(2) + self.m[0][2].powi(2)).sqrt();
        let sy = (self.m[1][0].powi(2) + self.m[1][1].powi(2) + self.m[1][2].powi(2)).sqrt();
        let sz = (self.m[2][0].powi(2) + self.m[2][1].powi(2) + self.m[2][2].powi(2)).sqrt();
        (sx + sy + sz) / 3.0
    }

    fn euler_xyz_degrees(&self) -> (f32, f32, f32) {
        let m = &self.m;
        let sy = (m[0][2].powi(2) + m[2][2].powi(2)).sqrt();
        let ry = m[0][2].atan2(sy).to_degrees();
        let rx = (-m[1][2]).atan2(m[1][1].max(1e-8)).to_degrees();
        let rz = (-m[0][1]).atan2(m[0][0].max(1e-8)).to_degrees();
        (rx, ry, rz)
    }
}

fn parse_primary_matrix(text: &str) -> Option<Matrix4> {
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("UT_DMatrix4") else {
            continue;
        };
        let vals: Vec<f32> = rest
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if vals.len() < 16 {
            continue;
        }
        let mut m = [[0.0f32; 4]; 4];
        for (i, v) in vals.iter().take(16).enumerate() {
            m[i / 4][i % 4] = *v;
        }
        return Some(Matrix4 { m });
    }
    None
}
