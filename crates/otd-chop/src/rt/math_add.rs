pub fn mix_add_scaled(a: &[f32], b: &[f32], gain: f32, out: &mut [f32]) {
    let n = out.len().min(a.len()).min(b.len());
    for i in 0..n {
        out[i] = a[i] + b[i] * gain;
    }
    for sample in out.iter_mut().skip(n) {
        *sample = 0.0;
    }
}

#[cfg(test)]
#[path = "../tests/rt_math_add__tests.rs"]
mod tests;
