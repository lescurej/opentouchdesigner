use std::collections::HashMap;

use crate::{Geometry, Point, Topology};

use super::types::{Detail, Prim};

pub(crate) fn geometry_from(detail: Detail) -> Result<Geometry, String> {
    let only_open = !detail.prims.is_empty()
        && detail
            .prims
            .iter()
            .all(|p| matches!(p, Prim::Poly { closed: false, .. }));
    if only_open {
        if let Some((points, indices)) = flat_lattice_shell(&detail.points) {
            return Ok(Geometry {
                points,
                indices,
                topology: Topology::Triangles,
                ..Default::default()
            });
        }
        let mut indices = Vec::new();
        for prim in &detail.prims {
            if let Prim::Poly { verts, .. } = prim {
                for pair in verts.windows(2) {
                    indices.extend_from_slice(&[pair[0], pair[1]]);
                }
            }
        }
        return Ok(Geometry {
            points: detail.points,
            indices,
            topology: Topology::Lines,
            ..Default::default()
        });
    }

    let mut indices = Vec::new();
    for prim in &detail.prims {
        match prim {
            Prim::Poly {
                closed: true,
                verts,
            } => indices.extend(fan_triangulate(verts)),
            Prim::Mesh {
                cols,
                rows,
                conn,
                verts,
            } => {
                indices.extend(mesh_tris(*cols, *rows, *conn, verts));
            }
            Prim::Poly { closed: false, .. } => {}
        }
    }
    if indices.is_empty() {
        return Ok(Geometry {
            points: detail.points,
            topology: Topology::Points,
            ..Default::default()
        });
    }
    let mut points = detail.points;
    if !detail.has_n {
        facet_normals(&mut points, &indices);
    }
    Ok(Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    })
}

fn fan_triangulate(verts: &[u32]) -> Vec<u32> {
    let mut tris = Vec::new();
    if verts.len() < 3 {
        return tris;
    }
    for i in 1..verts.len() - 1 {
        let (a, b, c) = (verts[0], verts[i], verts[i + 1]);
        if a != b && b != c && a != c {
            tris.extend_from_slice(&[a, b, c]);
        }
    }
    tris
}

/// `Mesh` primitive, vertices in row-major order (`column + row * columns`).
fn mesh_tris(cols: usize, rows: usize, conn: i32, verts: &[u32]) -> Vec<u32> {
    if cols < 2 || rows < 2 || matches!(conn, 0..=2) {
        return Vec::new();
    }
    let mut tris = Vec::new();
    for r in 0..rows - 1 {
        for c in 0..cols - 1 {
            let i = c + r * cols;
            let Some(&[a, b]) = verts.get(i..i + 2) else {
                continue;
            };
            let j = c + (r + 1) * cols;
            let Some(&[d, e]) = verts.get(j..j + 2) else {
                continue;
            };
            if conn == 6 {
                tris.extend_from_slice(&[a, e, b, a, d, e]);
            } else {
                tris.extend_from_slice(&[a, b, e, a, e, d]);
            }
        }
    }
    tris
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

/// Open grid lines on a regular lattice share one normal per corner. Split a
/// copy of each sample onto its face so the face stays flat.
fn flat_lattice_shell(points: &[Point]) -> Option<(Vec<Point>, Vec<u32>)> {
    if points.len() < 8 {
        return None;
    }
    let xs = unique_axis(points.iter().map(|p| p.position[0]));
    let ys = unique_axis(points.iter().map(|p| p.position[1]));
    let zs = unique_axis(points.iter().map(|p| p.position[2]));
    if xs.len() < 2 || ys.len() < 2 || zs.len() < 2 {
        return None;
    }
    let mut lattice: HashMap<(usize, usize, usize), [f32; 3]> = HashMap::new();
    for p in points {
        let (Some(ix), Some(iy), Some(iz)) = (
            axis_slot(&xs, p.position[0]),
            axis_slot(&ys, p.position[1]),
            axis_slot(&zs, p.position[2]),
        ) else {
            continue;
        };
        lattice.entry((ix, iy, iz)).or_insert(p.position);
    }
    if lattice.len() != xs.len() * ys.len() * zs.len() {
        return None;
    }
    let (nx, ny, nz) = (xs.len(), ys.len(), zs.len());
    let mut pts = Vec::new();
    let mut tris = Vec::new();
    let mut quads = Vec::new();
    for y in 0..ny - 1 {
        for z in 0..nz - 1 {
            quads.push([(0, y, z), (0, y, z + 1), (0, y + 1, z + 1), (0, y + 1, z)]);
        }
    }
    emit_flat_face(&mut pts, &mut tris, &lattice, &quads, [-1.0, 0.0, 0.0]);
    quads.clear();
    for y in 0..ny - 1 {
        for z in 0..nz - 1 {
            quads.push([
                (nx - 1, y, z),
                (nx - 1, y + 1, z),
                (nx - 1, y + 1, z + 1),
                (nx - 1, y, z + 1),
            ]);
        }
    }
    emit_flat_face(&mut pts, &mut tris, &lattice, &quads, [1.0, 0.0, 0.0]);
    quads.clear();
    for x in 0..nx - 1 {
        for z in 0..nz - 1 {
            quads.push([(x, 0, z), (x + 1, 0, z), (x + 1, 0, z + 1), (x, 0, z + 1)]);
        }
    }
    emit_flat_face(&mut pts, &mut tris, &lattice, &quads, [0.0, -1.0, 0.0]);
    quads.clear();
    for x in 0..nx - 1 {
        for z in 0..nz - 1 {
            quads.push([
                (x, ny - 1, z),
                (x, ny - 1, z + 1),
                (x + 1, ny - 1, z + 1),
                (x + 1, ny - 1, z),
            ]);
        }
    }
    emit_flat_face(&mut pts, &mut tris, &lattice, &quads, [0.0, 1.0, 0.0]);
    quads.clear();
    for x in 0..nx - 1 {
        for y in 0..ny - 1 {
            quads.push([(x, y, 0), (x, y + 1, 0), (x + 1, y + 1, 0), (x + 1, y, 0)]);
        }
    }
    emit_flat_face(&mut pts, &mut tris, &lattice, &quads, [0.0, 0.0, -1.0]);
    quads.clear();
    for x in 0..nx - 1 {
        for y in 0..ny - 1 {
            quads.push([
                (x, y, nz - 1),
                (x + 1, y, nz - 1),
                (x + 1, y + 1, nz - 1),
                (x, y + 1, nz - 1),
            ]);
        }
    }
    emit_flat_face(&mut pts, &mut tris, &lattice, &quads, [0.0, 0.0, 1.0]);
    if tris.is_empty() {
        None
    } else {
        Some((pts, tris))
    }
}

fn emit_flat_face(
    pts: &mut Vec<Point>,
    tris: &mut Vec<u32>,
    lattice: &HashMap<(usize, usize, usize), [f32; 3]>,
    quads: &[[(usize, usize, usize); 4]],
    normal: [f32; 3],
) {
    let mut local: HashMap<(usize, usize, usize), u32> = HashMap::new();
    for corners in quads {
        let mut ids = [0u32; 4];
        let mut ok = true;
        for (k, corner) in corners.iter().enumerate() {
            let Some(&position) = lattice.get(corner) else {
                ok = false;
                break;
            };
            ids[k] = *local.entry(*corner).or_insert_with(|| {
                let id = pts.len() as u32;
                pts.push(Point {
                    position,
                    normal,
                    ..Point::default()
                });
                id
            });
        }
        if ok {
            tris.extend_from_slice(&[ids[0], ids[1], ids[2], ids[0], ids[2], ids[3]]);
        }
    }
}

fn unique_axis(values: impl Iterator<Item = f32>) -> Vec<f32> {
    let mut v: Vec<f32> = values.filter(|x| x.is_finite()).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = Vec::new();
    for x in v {
        if out.last().is_none_or(|p: &f32| (x - *p).abs() > 1.0e-4) {
            out.push(x);
        }
    }
    out
}

fn axis_slot(axis: &[f32], v: f32) -> Option<usize> {
    axis.iter().position(|u| (u - v).abs() <= 1.0e-4)
}
