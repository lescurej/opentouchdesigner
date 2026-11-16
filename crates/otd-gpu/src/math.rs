//! Just enough 4x4 matrix maths for the render path.
//!
//! A dependency for this would be reasonable; twenty lines that we can read
//! and that match the renderer's conventions exactly is more reasonable
//! still. Column-major, `wgpu`'s 0..1 depth range.

pub type Mat4 = [[f32; 4]; 4];

pub const IDENTITY: Mat4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

pub fn mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut out = [[0.0f32; 4]; 4];
    for (col, out_col) in out.iter_mut().enumerate() {
        for (row, cell) in out_col.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][row] * b[col][k]).sum();
        }
    }
    out
}

pub fn translation(t: [f32; 3]) -> Mat4 {
    let mut m = IDENTITY;
    m[3][0] = t[0];
    m[3][1] = t[1];
    m[3][2] = t[2];
    m
}

pub fn scaling(s: [f32; 3]) -> Mat4 {
    let mut m = IDENTITY;
    m[0][0] = s[0];
    m[1][1] = s[1];
    m[2][2] = s[2];
    m
}

/// XYZ Euler rotation in degrees, matching the SOP Transform's order.
pub fn rotation(deg: [f32; 3]) -> Mat4 {
    let (sx, cx) = deg[0].to_radians().sin_cos();
    let (sy, cy) = deg[1].to_radians().sin_cos();
    let (sz, cz) = deg[2].to_radians().sin_cos();

    let rx: Mat4 = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cx, sx, 0.0],
        [0.0, -sx, cx, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let ry: Mat4 = [
        [cy, 0.0, -sy, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sy, 0.0, cy, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let rz: Mat4 = [
        [cz, sz, 0.0, 0.0],
        [-sz, cz, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    mul(rz, mul(ry, rx))
}

/// Translate ∘ rotate ∘ scale, the order every transform parameter page uses.
pub fn trs(t: [f32; 3], r: [f32; 3], s: [f32; 3]) -> Mat4 {
    mul(translation(t), mul(rotation(r), scaling(s)))
}

/// Right-handed perspective onto wgpu's 0..1 depth range.
pub fn perspective(fov_deg: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_deg.to_radians() * 0.5).tan();
    let mut m = [[0.0f32; 4]; 4];
    m[0][0] = f / aspect.max(1e-6);
    m[1][1] = f;
    m[2][2] = far / (near - far);
    m[2][3] = -1.0;
    m[3][2] = near * far / (near - far);
    m
}

pub fn orthographic(height: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let h = height.max(1e-6) * 0.5;
    let w = h * aspect.max(1e-6);
    let mut m = IDENTITY;
    m[0][0] = 1.0 / w;
    m[1][1] = 1.0 / h;
    m[2][2] = 1.0 / (near - far);
    m[3][2] = near / (near - far);
    m
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-9 {
        return [0.0, 0.0, 1.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// A view matrix for a camera at `eye` looking at `target`.
pub fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> Mat4 {
    let f = normalize([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    let s = normalize(cross(f, up));
    let u = cross(s, f);
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
    ]
}

/// The inverse of a transform built by [`trs`] — a camera's view matrix is
/// the inverse of its placement in the world.
pub fn inverse_trs(t: [f32; 3], r: [f32; 3], s: [f32; 3]) -> Mat4 {
    let inv_scale = scaling([
        1.0 / if s[0].abs() < 1e-9 { 1.0 } else { s[0] },
        1.0 / if s[1].abs() < 1e-9 { 1.0 } else { s[1] },
        1.0 / if s[2].abs() < 1e-9 { 1.0 } else { s[2] },
    ]);
    // A rotation's inverse is its transpose.
    let rot = rotation(r);
    let mut inv_rot = IDENTITY;
    for (i, col) in inv_rot.iter_mut().enumerate().take(3) {
        for (j, cell) in col.iter_mut().enumerate().take(3) {
            *cell = rot[j][i];
        }
    }
    mul(inv_scale, mul(inv_rot, translation([-t[0], -t[1], -t[2]])))
}

#[cfg(test)]
#[path = "tests/math__tests.rs"]
mod tests;
