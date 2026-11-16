// Noise TOP — Perlin 2D in 0–1 texture space (TouchDesigner default type).
//
// p0 = (period, harmonics, roughness, exponent)
// p1 = (translate.x, translate.y, translate.z, monochrome)
// p2 = (amplitude, offset, seed, harmonic spread)
// p3 = (harmonic gain, 0, 0, 0)
// Harmonic spread is unused: it only applies to Harmonic Summation, which this
// cook does not implement. Harmonics is the number of extra octaves; 0 is the
// base octave alone.

fn hash_u(x: u32) -> u32 {
  var h = x;
  h ^= h >> 16u;
  h *= 0x7feb352du;
  h ^= h >> 15u;
  h *= 0x846ca68bu;
  h ^= h >> 16u;
  return h;
}

fn grad(h: u32, x: f32, y: f32) -> f32 {
  let sx = select(-x, x, (h & 1u) == 0u);
  let sy = select(-y, y, (h & 2u) == 0u);
  return select(sx + sy, sx - sy, (h & 4u) == 0u);
}

fn perlin2(p: vec2<f32>, seed: u32) -> f32 {
  let i = vec2<i32>(floor(p));
  let f = fract(p);
  let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  let aa = hash_u(bitcast<u32>(i.x) ^ hash_u(bitcast<u32>(i.y) ^ seed));
  let ba = hash_u(bitcast<u32>(i.x + 1) ^ hash_u(bitcast<u32>(i.y) ^ seed));
  let ab = hash_u(bitcast<u32>(i.x) ^ hash_u(bitcast<u32>(i.y + 1) ^ seed));
  let bb = hash_u(bitcast<u32>(i.x + 1) ^ hash_u(bitcast<u32>(i.y + 1) ^ seed));
  let x1 = mix(grad(aa, f.x, f.y), grad(ba, f.x - 1.0, f.y), u.x);
  let x2 = mix(grad(ab, f.x, f.y - 1.0), grad(bb, f.x - 1.0, f.y - 1.0), u.x);
  return mix(x1, x2, u.y) * 1.41421356;
}

fn fbm(p: vec2<f32>, harmonics: i32, roughness: f32, gain: f32, seed: u32) -> f32 {
  var sum = perlin2(p, seed);
  var amp = roughness * gain;
  var q = p * 2.0;
  let n = max(harmonics, 0) + 1;
  for (var i = 1; i < n; i = i + 1) {
    sum = sum + amp * perlin2(q, seed);
    amp = amp * roughness;
    q = q * 2.0;
  }
  return sum;
}

fn shape(n: f32, exponent: f32, amplitude: f32, offset: f32) -> f32 {
  let c = clamp(n, -1.0, 1.0);
  return sign(c) * pow(abs(c), exponent) * amplitude + offset;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
  let period = max(U.p0.x, 1e-4);
  let harmonics = i32(U.p0.y);
  let roughness = clamp(U.p0.z, 0.0, 8.0);
  let exponent = max(U.p0.w, 1e-3);
  let gain = max(U.p3.x, 0.0);
  let seed = bitcast<u32>(i32(round(U.p2.z)));
  let aspect = U.res.y / max(U.res.x, 1.0);
  let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
  let p = vec2<f32>(uv.x, uv.y * aspect) / period + U.p1.xy;

  let mono = U.p1.w > 0.5;
  var rgb: vec3<f32>;
  if (mono) {
    let n = shape(fbm(p, harmonics, roughness, gain, seed), exponent, U.p2.x, U.p2.y);
    rgb = vec3<f32>(n);
  } else {
    rgb = vec3<f32>(
      shape(fbm(p, harmonics, roughness, gain, seed), exponent, U.p2.x, U.p2.y),
      shape(fbm(p, harmonics, roughness, gain, seed ^ 0x9e3779b9u), exponent, U.p2.x, U.p2.y),
      shape(fbm(p, harmonics, roughness, gain, seed ^ 0x85ebca6bu), exponent, U.p2.x, U.p2.y),
    );
  }
  return vec4<f32>(rgb, 1.0);
}
