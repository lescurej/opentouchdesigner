// Analyze TOP — 2x2 reduction. The engine repeats this until the image is 1x1.
// p0 = (source width, source height, function: 0 average / 1 minimum / 2 maximum, 0)

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
  let src = vec2<i32>(U.p0.xy);
  let op = i32(U.p0.z + 0.5);
  let pix = vec2<i32>(in.uv * max(U.res.xy, vec2<f32>(1.0)));
  let origin = pix * 2;
  var sum = vec4<f32>(0.0);
  var count = 0.0;
  var lo = vec4<f32>(1e9);
  var hi = vec4<f32>(-1e9);
  var any = false;
  for (var y = 0; y < 2; y = y + 1) {
    for (var x = 0; x < 2; x = x + 1) {
      let p = origin + vec2<i32>(x, y);
      if (p.x < src.x && p.y < src.y) {
        let s = textureLoad(tex0, p, 0);
        sum = sum + s;
        count = count + 1.0;
        lo = min(lo, s);
        hi = max(hi, s);
        any = true;
      }
    }
  }
  if (!any) {
    return vec4<f32>(0.0);
  }
  if (op == 1) {
    return lo;
  }
  if (op == 2) {
    return hi;
  }
  return sum / max(count, 1.0);
}
