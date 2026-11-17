use super::*;

#[test]
fn feedback_chop_integrates_one_frame_behind() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    p.set(src, "name0", Value::Str("v".into()));
    p.set(src, "value0", Value::Float(1.0));
    let math = p.add("mathCHOP", "math1");
    let fb = p.add("feedbackCHOP", "fb1");
    p.set(fb, "target", Value::Str("math1".into()));
    p.graph.connect(src, math, 0).unwrap();
    p.graph.connect(fb, math, 1).unwrap();

    p.run(math, 1);
    assert_eq!(p.value(math, "v"), 1.0);
    assert!(p.data(fb).channels.is_empty());

    p.run(math, 1);
    assert_eq!(p.value(fb, "v"), 1.0);
    assert_eq!(p.value(math, "v"), 2.0);

    p.run(math, 1);
    assert_eq!(p.value(fb, "v"), 2.0);
    assert_eq!(p.value(math, "v"), 3.0);
}

#[test]
fn feedback_chop_lags_an_upstream_target() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    p.set(src, "value0", Value::Float(4.0));
    let fb = p.add("feedbackCHOP", "fb1");
    p.set(fb, "target", Value::Str("/src".into()));

    let step = |p: &mut Patch| {
        p.cook
            .cook_frame(&p.graph, &[src, fb], &p.time, &mut p.host)
            .unwrap();
        p.host.end_frame();
        p.time.advance_nominal_frame();
    };

    step(&mut p);
    assert!(p.data(fb).channels.is_empty());
    step(&mut p);
    assert_eq!(p.value(fb, "chan1"), 4.0);

    p.set(src, "value0", Value::Float(9.0));
    step(&mut p);
    assert_eq!(p.value(fb, "chan1"), 4.0);
    step(&mut p);
    assert_eq!(p.value(fb, "chan1"), 9.0);
}

#[test]
fn feedback_chop_reset_drops_the_held_channels() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    p.set(src, "value0", Value::Float(4.0));
    let fb = p.add("feedbackCHOP", "fb1");
    p.set(fb, "target", Value::Str("src".into()));

    let step = |p: &mut Patch| {
        p.cook
            .cook_frame(&p.graph, &[src, fb], &p.time, &mut p.host)
            .unwrap();
        p.host.end_frame();
        p.time.advance_nominal_frame();
    };
    step(&mut p);
    step(&mut p);
    assert_eq!(p.value(fb, "chan1"), 4.0);

    p.set(fb, "reset", Value::Bool(true));
    step(&mut p);
    assert!(p.data(fb).channels.is_empty());

    p.set(fb, "reset", Value::Bool(false));
    step(&mut p);
    assert!(p.data(fb).channels.is_empty());
    step(&mut p);
    assert_eq!(p.value(fb, "chan1"), 4.0);
}
