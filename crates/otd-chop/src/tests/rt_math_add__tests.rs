use super::*;

#[test]
fn add_with_gain() {
    let a = [1.0_f32, 2.0];
    let b = [0.5, 1.0];
    let mut out = [0.0; 2];
    mix_add_scaled(&a, &b, 2.0, &mut out);
    assert!((out[0] - 2.0).abs() < 1e-6);
    assert!((out[1] - 4.0).abs() < 1e-6);
}
