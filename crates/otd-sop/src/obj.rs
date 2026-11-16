//! Minimal Wavefront OBJ reader for `fileinSOP` / `importselectSOP`.

use std::fs;
use std::path::Path;

use crate::{Geometry, Point, Topology};

#[derive(Default)]
struct ObjObject {
    name: String,
    faces: Vec<[usize; 3]>,
}

/// Load an entire OBJ, or one object/group selected by `selection`.
///
/// `selection` is the TouchDesigner-style path (`/Model/Group1/...`) or a
/// short name. Empty selection merges all objects.
pub fn load(path: &Path, selection: &str) -> Result<Geometry, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("obj: {e}"))?;
    parse(&text, selection)
}

pub fn parse(text: &str, selection: &str) -> Result<Geometry, String> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut objects: Vec<ObjObject> = Vec::new();
    let mut current = ObjObject {
        name: "default".into(),
        ..Default::default()
    };

    let want = selection.trim();
    let want_leaf = want.rsplit('/').next().unwrap_or(want);

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let tag = parts.next().unwrap_or("");
        match tag {
            "v" => {
                let x: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let y: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let z: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                positions.push([x, y, z]);
            }
            "vn" => {
                let x: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let y: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let z: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                normals.push([x, y, z]);
            }
            "vt" => {
                let u: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                let v: f32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                uvs.push([u, v]);
            }
            "f" => {
                let mut face = [0usize; 3];
                for (i, tok) in parts.take(3).enumerate() {
                    let vi = tok
                        .split('/')
                        .next()
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(0);
                    let idx = vi.unsigned_abs() as usize;
                    face[i] = idx.saturating_sub(1);
                }
                current.faces.push(face);
            }
            "o" | "g" => {
                if !current.faces.is_empty() || !current.name.is_empty() {
                    objects.push(std::mem::take(&mut current));
                }
                current.name = parts.collect::<Vec<_>>().join(" ");
                if current.name.is_empty() {
                    current.name = "default".into();
                }
            }
            _ => {}
        }
    }
    if !current.faces.is_empty() || !objects.is_empty() {
        objects.push(current);
    }

    let picked: Vec<&ObjObject> = if want.is_empty() {
        objects.iter().collect()
    } else {
        objects
            .iter()
            .filter(|o| {
                o.name == want
                    || o.name.ends_with(want_leaf)
                    || format!("/{}", o.name.replace(' ', "/")) == want
                    || want.ends_with(&o.name)
            })
            .collect()
    };

    if picked.is_empty() && !objects.is_empty() && want.is_empty() {
        return Ok(Geometry::default());
    }
    if picked.is_empty() {
        return Err(format!("obj: no object matching `{selection}`"));
    }

    let mut points = Vec::new();
    let mut indices = Vec::new();
    let mut remap: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();

    for obj in picked {
        for tri in &obj.faces {
            for &pi in tri {
                if pi >= positions.len() {
                    continue;
                }
                let out = *remap.entry(pi).or_insert_with(|| {
                    let n = [0.0, 1.0, 0.0];
                    let uv = if pi < uvs.len() { uvs[pi] } else { [0.0, 0.0] };
                    let idx = points.len() as u32;
                    points.push(Point {
                        position: positions[pi],
                        normal: n,
                        uv,
                        color: [1.0; 4],
                    });
                    idx
                });
                indices.push(out);
            }
        }
    }

    if points.is_empty() {
        return Ok(Geometry::default());
    }
    facet_normals(&mut points, &indices);
    Ok(Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    })
}

fn facet_normals(points: &mut [Point], indices: &[u32]) {
    for tri in indices.chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let a = points[tri[0] as usize].position;
        let b = points[tri[1] as usize].position;
        let c = points[tri[2] as usize].position;
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let mut n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-8);
        n = [n[0] / len, n[1] / len, n[2] / len];
        for &i in tri {
            points[i as usize].normal = n;
        }
    }
}

#[cfg(test)]
#[path = "tests/obj__tests.rs"]
mod tests;
