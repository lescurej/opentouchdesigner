use super::*;

#[test]
fn a_parameter_exports_from_a_chop_channel() {
    let mut p = Patch::new();
    let lfo = p.add("lfoCHOP", "lfo1");
    let driven = p.add("constantCHOP", "driven");
    p.set(lfo, "frequency", Value::Float(1.0));

    // `driven.value0` follows `/lfo1:lfo` rather than its own constant.
    p.graph
        .set_expression(driven, "value0", &channel_expression("/lfo1", "lfo"))
        .unwrap();

    // Pulling only `driven` must drag the LFO in as a dependency.
    p.run(driven, 15);
    assert!(
        p.cook.cook_count(lfo) > 0,
        "the export source must have cooked"
    );
    let exported = p.value(driven, "chan1");
    let source = p.value(lfo, "lfo");
    assert!(
        (exported - source).abs() < 1e-5,
        "exported {exported} should track the channel {source}"
    );
    assert!(
        p.cook.is_time_dependent(driven),
        "an export from an animated CHOP animates the consumer"
    );
}

#[test]
fn a_dangling_export_leaves_the_constant_in_place() {
    let mut p = Patch::new();
    let driven = p.add("constantCHOP", "driven");
    p.set(driven, "value0", Value::Float(0.5));
    p.graph
        .set_expression(
            driven,
            "value0",
            &channel_expression("/nonexistent", "chan1"),
        )
        .unwrap();
    p.run(driven, 1);
    assert_eq!(p.value(driven, "chan1"), 0.5);
}

#[test]
fn a_bind_reads_another_operators_parameter() {
    let mut p = Patch::new();
    let source = p.add("constantCHOP", "source");
    let follower = p.add("constantCHOP", "follower");
    p.set(source, "value0", Value::Float(7.0));
    p.graph
        .set_expression(
            follower,
            "value0",
            &parameter_expression("/source", "value0"),
        )
        .unwrap();

    p.run(follower, 1);
    assert_eq!(p.value(follower, "chan1"), 7.0);

    p.set(source, "value0", Value::Float(9.0));
    p.run(follower, 1);
    assert_eq!(p.value(follower, "chan1"), 9.0);
}

#[test]
fn a_keyframed_curve_drives_a_parameter_through_export() {
    let mut p = Patch::new();
    let a = p.add("animationCHOP", "anim1");
    p.set(
        a,
        "keys",
        Value::Str("gain 0 0 linear\ngain 2 1 linear\n".into()),
    );
    let m = p.add("mathCHOP", "math1");
    p.graph.connect(a, m, 0).unwrap();

    // The point of making keyframes a CHOP: everything downstream — filters,
    // maths, Export to a parameter — works on them with no new mechanism.
    p.set(m, "gain", Value::Float(100.0));
    run_to(&mut p, m, 1.0);
    assert!(
        (p.value(m, "gain") - 50.0).abs() < 2.0,
        "{}",
        p.value(m, "gain")
    );
}

#[test]
fn an_expression_chop_sees_the_sample_and_its_position() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let e = p.add("expressionCHOP", "expr1");
    p.graph.connect(pat, e, 0).unwrap();
    p.set(pat, "length", Value::Int(10));
    p.set(pat, "type", Value::Str("ramp".into()));

    // `i` is the sample index and `n` the count, so this rebuilds the ramp
    // from scratch and must match a straight 0..1 sweep.
    p.set(e, "expr", Value::Str("i / (n - 1)".into()));
    p.run(e, 1);
    let out = &p.data(e).nth(0).unwrap().samples;
    assert_eq!(out.len(), 10);
    assert!((out[0] - 0.0).abs() < 1e-5);
    assert!((out[9] - 1.0).abs() < 1e-5);

    // `v` is the incoming sample.
    p.set(e, "expr", Value::Str("v * 2 + 1".into()));
    p.run(e, 1);
    let src = p.data(pat).nth(0).unwrap().copy_samples();
    let out = &p.data(e).nth(0).unwrap().samples;
    for (i, v) in src.iter().enumerate() {
        assert!((out[i] - (v * 2.0 + 1.0)).abs() < 1e-4, "sample {i}");
    }
}

#[test]
fn an_expression_stored_as_a_parameter_expression_still_sees_v() {
    // Typing `v * 2` used to flip the field into parameter-expression mode.
    // That language has no sample, so the panel reported `v` as unknown and
    // the cook kept the previous constant. The text is the per-sample formula.
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let e = p.add("expressionCHOP", "expr1");
    p.graph.connect(c, e, 0).unwrap();
    p.set(c, "value0", Value::Float(3.0));
    p.graph.set_expression(e, "expr", "v * 2").unwrap();
    p.run(e, 1);
    assert!(
        (p.value(e, "chan1") - 6.0).abs() < 1e-4,
        "v is the sample, got {}",
        p.value(e, "chan1")
    );
}

#[test]
fn a_half_typed_expression_holds_the_input_rather_than_emptying_it() {
    // Typing `v *` on the way to `v * 2` must not black out a running patch,
    // which is the same promise the GLSL TOP makes for a shader mid-edit.
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let e = p.add("expressionCHOP", "expr1");
    p.graph.connect(c, e, 0).unwrap();
    p.set(c, "value0", Value::Float(0.75));
    p.set(e, "expr", Value::Str("v *".into()));
    p.run(e, 1);

    assert_eq!(p.value(e, "chan1"), 0.75);
}

#[test]
fn an_expression_can_reach_the_clock() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let e = p.add("expressionCHOP", "expr1");
    p.graph.connect(c, e, 0).unwrap();
    p.set(e, "expr", Value::Str("frame".into()));

    p.run(e, 5);
    let first = p.value(e, "chan1");
    p.run(e, 5);
    assert!(
        p.value(e, "chan1") > first,
        "the frame number should be advancing"
    );
}
