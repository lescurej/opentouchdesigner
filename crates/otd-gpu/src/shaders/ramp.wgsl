// Ramp TOP — coordinate field + 1×N LUT in input 2 (tex1).
// p0 = (type, phase, period, interp)
// p1, p2 = fallback colours (also used when building the LUT on CPU)
// p3 = (extend left, extend right, position.x, position.y)
// p4 = (antialias, fit aspect, tension, 0)

const RAMP_PI: f32 = 3.14159265;

// 0 hold · 1 repeat · 2 mirror · 3 black · 4 zero
fn extend_ramp(t: f32, left: i32, right: i32) -> f32 {
  if (t >= 0.0 && t <= 1.0) {
    return t;
  }
  if (left == 1 || right == 1) {
    return fract(t);
  }
  if (left == 2 || right == 2) {
    let m = abs(fract(t * 0.5) * 2.0 - 1.0);
    return 1.0 - m;
  }
  if (t < 0.0 && left == 0) {
    return 0.0;
  }
  if (t > 1.0 && right == 0) {
    return 1.0;
  }
  return clamp(t, 0.0, 1.0);
}

fn radial_vector(uv: vec2<f32>, pos: vec2<f32>, aspect: f32, fit: i32) -> vec2<f32> {
  var dx = uv.x - pos.x;
  var dy = uv.y - pos.y;
  if (fit == 0) {
    dx = dx * aspect;
  } else if (fit == 1) {
    // fit horizontal — no x stretch
  } else if (fit == 2) {
    dy = dy / max(aspect, 1e-5);
  } else {
    if (aspect >= 1.0) {
      dx = dx * aspect;
    } else {
      dy = dy / max(aspect, 1e-5);
    }
  }
  return vec2<f32>(dx, dy);
}

fn sample_lut(t: f32, aa: f32) -> vec4<f32> {
  if (aa <= 0.0) {
    return sample1(vec2<f32>(clamp(t, 0.0, 1.0), 0.5));
  }
  let w = max(fwidth(t) * aa, 1.0 / 512.0);
  let c0 = sample1(vec2<f32>(clamp(t - w, 0.0, 1.0), 0.5));
  let c1 = sample1(vec2<f32>(clamp(t, 0.0, 1.0), 0.5));
  let c2 = sample1(vec2<f32>(clamp(t + w, 0.0, 1.0), 0.5));
  return (c0 + c1 + c2) / 3.0;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
  let kind = i32(U.p0.x + 0.5);
  let aspect = U.res.x / max(U.res.y, 1.0);
  let pos = U.p3.zw;
  let fit = i32(U.p4.y + 0.5);
  let aa = U.p4.x;
  var coord: f32;
  if (kind == 1) {
    coord = 1.0 - in.uv.y;
  } else if (kind == 2) {
    let d = radial_vector(in.uv, pos, aspect, fit);
    coord = length(d) * 2.0;
  } else if (kind == 3) {
    let d = radial_vector(in.uv, pos, aspect, fit);
    coord = atan2(d.y, d.x) / (2.0 * RAMP_PI) + 0.5;
  } else {
    coord = in.uv.x;
  }

  let period = max(U.p0.z, 1e-5);
  let raw = coord / period + fract(U.p0.y);
  let left = i32(U.p3.x + 0.5);
  let right = i32(U.p3.y + 0.5);

  if (raw < 0.0 && left == 3) {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
  }
  if (raw > 1.0 && right == 3) {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
  }
  if (raw < 0.0 && left == 4) {
    return vec4<f32>(0.0);
  }
  if (raw > 1.0 && right == 4) {
    return vec4<f32>(0.0);
  }

  let t = extend_ramp(raw, left, right);
  let use_aa = kind >= 2;
  let strength = select(0.0, aa, use_aa);
  return sample_lut(t, strength);
}
