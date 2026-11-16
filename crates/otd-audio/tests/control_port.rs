use otd_audio::ControlPort;

#[test]
fn ramps_toward_target_within_one_block() {
    let p = ControlPort::new(0.0);
    p.set(1.0);
    let ramp = p.ramp(4);
    assert!((ramp.at(0) - 0.25).abs() < 1e-6);
    assert!((ramp.at(3) - 1.0).abs() < 1e-6);
    let flat = p.ramp(4);
    assert_eq!(flat.step, 0.0);
    assert_eq!(flat.start, 1.0);
}

#[test]
fn ignores_non_finite_values() {
    let p = ControlPort::new(0.5);
    p.set(f32::NAN);
    assert_eq!(p.target(), 0.5);
}
