// Switch TOP — chooses between two inputs, or blends across the boundary so
// an animated index crossfades rather than pops.
// p0 = (index, blend, 0, 0)

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
  let a = sample0(in.uv);
  let b = sample1(in.uv);
  var t = U.p0.x;
  if (U.p0.y < 0.5) {
    t = select(0.0, 1.0, t >= 1.0);
  } else {
    t = clamp(t, 0.0, 1.0);
  }
  return mix(a, b, t);
}
