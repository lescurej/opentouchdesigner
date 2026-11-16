// Blur TOP — separable Gaussian. The engine runs this twice: horizontal into
// a scratch texture, then vertical into the output.
//
// p0 = (radius in output pixels, direction 0 = horizontal / 1 = vertical,
//       pre-shrink, 0)
// Pre-shrink is a box downsample of the original input, applied on the
// horizontal pass only. The vertical pass blurs the already-shrunk scratch.

fn load_texel(p: vec2<i32>) -> vec4<f32> {
  let size = vec2<i32>(textureDimensions(tex0));
  let c = clamp(p, vec2<i32>(0), size - vec2<i32>(1, 1));
  return textureLoad(tex0, c, 0);
}

fn sample_input(uv: vec2<f32>, shrink: i32) -> vec4<f32> {
  let n = clamp(shrink, 1, 16);
  let out_size = vec2<i32>(max(U.res.xy, vec2<f32>(1.0)));
  let pix = clamp(vec2<i32>(floor(uv * vec2<f32>(out_size))), vec2<i32>(0), out_size - vec2<i32>(1, 1));
  if (n <= 1) {
    return load_texel(pix);
  }
  let origin = pix * n;
  var sum = vec4<f32>(0.0);
  for (var y = 0; y < 16; y = y + 1) {
    if (y >= n) { break; }
    for (var x = 0; x < 16; x = x + 1) {
      if (x >= n) { break; }
      sum = sum + load_texel(origin + vec2<i32>(x, y));
    }
  }
  return sum / f32(n * n);
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
  let shrink = select(1, i32(U.p0.z + 0.5), U.p0.y < 0.5);
  let radius = max(U.p0.x, 0.0);
  if (radius < 0.5) {
    return sample_input(in.uv, shrink);
  }
  var dir = vec2<f32>(U.res.z, 0.0);
  if (U.p0.y > 0.5) {
    dir = vec2<f32>(0.0, U.res.w);
  }

  let taps = 16;
  let sigma = radius * 0.5;
  let step = radius / f32(taps);

  var sum = sample_input(in.uv, shrink);
  var weight = 1.0;
  for (var i = 1; i <= taps; i = i + 1) {
    let o = f32(i) * step;
    let w = exp(-(o * o) / (2.0 * sigma * sigma + 1e-6));
    sum = sum + w * (sample_input(in.uv + dir * o, shrink) + sample_input(in.uv - dir * o, shrink));
    weight = weight + 2.0 * w;
  }
  return sum / weight;
}
