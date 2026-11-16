// Threshold TOP.
// p0 = (threshold, softness, mode, invert)
// p1 = below colour
// p2 = above colour
// Softness 0 is a hard step: 0 below the threshold, 1 at and above it.

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
  let c = sample0(in.uv);

  let mode = i32(U.p0.z + 0.5);
  var key = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
  if (mode == 1) { key = max(max(c.r, c.g), c.b); }
  if (mode == 2) { key = c.a; }

  let soft = max(U.p0.y, 0.0);
  var t: f32;
  if (soft <= 1e-6) {
    t = select(0.0, 1.0, key >= U.p0.x);
  } else {
    t = smoothstep(U.p0.x - soft * 0.5, U.p0.x + soft * 0.5, key);
  }
  t = mix(t, 1.0 - t, clamp(U.p0.w, 0.0, 1.0));

  return mix(U.p1, U.p2, t);
}
