use super::*;

#[test]
fn an_animation_chop_follows_its_keys() {
    let mut p = Patch::new();
    let a = p.add("animationCHOP", "anim1");
    p.set(
        a,
        "keys",
        Value::Str("tx 0 0 linear\ntx 2 10 linear\nty 0 5 constant\n".into()),
    );

    // Two named channels, in name order, each reading its own curve.
    run_to(&mut p, a, 1.0);
    assert_eq!(p.data(a).names(), vec!["tx", "ty"]);
    assert!((p.value(a, "tx") - 5.0).abs() < 0.2, "{}", p.value(a, "tx"));
    assert_eq!(p.value(a, "ty"), 5.0);

    run_to(&mut p, a, 1.0);
    assert!(
        (p.value(a, "tx") - 10.0).abs() < 0.2,
        "{}",
        p.value(a, "tx")
    );
}

#[test]
fn an_animation_chop_can_loop_its_keyed_span() {
    let mut p = Patch::new();
    let a = p.add("animationCHOP", "anim1");
    p.set(a, "keys", Value::Str("a 0 0 linear\na 2 2 linear\n".into()));

    // The default holds after the last key, which is what an unlooped cue
    // should do when it finishes.
    run_to(&mut p, a, 5.0);
    assert_eq!(p.value(a, "a"), 2.0);

    // Looping wraps into the keyed span instead: five seconds into a
    // two-second curve is one second in.
    p.set(a, "play", Value::Str("loop".into()));
    p.run(a, 1);
    assert!(
        (p.value(a, "a") - 1.0).abs() < 0.1,
        "expected the loop to wrap, got {}",
        p.value(a, "a")
    );
}

#[test]
fn an_animation_chop_with_no_keys_still_produces_a_channel() {
    let mut p = Patch::new();
    let a = p.add("animationCHOP", "anim1");
    p.set(a, "keys", Value::Str(String::new()));
    p.run(a, 1);
    // Downstream operators should not have to special-case an empty CHOP just
    // because nobody has keyed anything yet.
    assert_eq!(p.data(a).names(), vec!["chan1"]);
    assert_eq!(p.value(a, "chan1"), 0.0);
}

#[test]
fn animation_speed_and_offset_shift_the_curve_in_time() {
    let mut p = Patch::new();
    let a = p.add("animationCHOP", "anim1");
    p.set(
        a,
        "keys",
        Value::Str("a 0 0 linear\na 10 10 linear\n".into()),
    );

    p.set(a, "speed", Value::Float(2.0));
    run_to(&mut p, a, 2.0);
    assert!((p.value(a, "a") - 4.0).abs() < 0.2, "{}", p.value(a, "a"));

    p.set(a, "speed", Value::Float(1.0));
    p.set(a, "offset", Value::Float(3.0));
    p.run(a, 1);
    assert!((p.value(a, "a") - 5.0).abs() < 0.2, "{}", p.value(a, "a"));
}
