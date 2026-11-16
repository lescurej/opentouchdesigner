use super::*;

#[test]
fn ltc_out_into_ltc_in_decodes_timeline_time() {
    let mut p = Patch::new();
    let out = p.add("ltcoutCHOP", "ltc1");
    let inp = p.add("ltcinCHOP", "ltc2");
    p.graph.connect(out, inp, 0).unwrap();
    p.set(out, "framerate", Value::Str("30".into()));
    p.set(out, "audiorate", Value::Float(48000.0));
    p.set(inp, "ratemenu", Value::Str("30".into()));
    p.set(inp, "rate", Value::Float(60.0));
    for _ in 0..300 {
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
        p.time.advance(1.0 / 30.0);
    }
    assert!(
        p.value(inp, "sec") >= 9.0,
        "expected ~10s decoded, got sec={}",
        p.value(inp, "sec")
    );
    assert!(p.value(inp, "total_frames") > 250.0);
}

#[test]
fn ltc_wire_locks_when_the_display_is_faster_than_the_project() {
    let mut p = Patch::new();
    let out = p.add("ltcoutCHOP", "ltcout1");
    let inp = p.add("ltcinCHOP", "ltcin1");
    p.graph.connect(out, inp, 0).unwrap();
    p.time.fps = 60.0;
    for _ in 0..240 {
        p.time.advance(1.0 / 120.0);
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
    }
    assert!(
        p.value(inp, "total_frames") > 20.0,
        "sec={} frame={} total={} status={:?}",
        p.value(inp, "sec"),
        p.value(inp, "frame"),
        p.value(inp, "total_frames"),
        p.host.engine.status("/ltcin1"),
    );
}

#[test]
fn ltc_wire_locks_when_the_project_is_slower_than_the_display() {
    let mut p = Patch::new();
    let out = p.add("ltcoutCHOP", "ltcout1");
    let inp = p.add("ltcinCHOP", "ltcin1");
    p.graph.connect(out, inp, 0).unwrap();
    p.time.fps = 30.0;
    for _ in 0..180 {
        p.time.advance_nominal_frame();
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
    }
    assert!(
        p.value(inp, "total_frames") > 20.0,
        "sec={} frame={} total={}",
        p.value(inp, "sec"),
        p.value(inp, "frame"),
        p.value(inp, "total_frames"),
    );
}

#[test]
fn ltc_out_below_nyquist_is_lifted_and_still_decodes() {
    let mut p = Patch::new();
    let out = p.add("ltcoutCHOP", "ltc1");
    let inp = p.add("ltcinCHOP", "ltc2");
    p.graph.connect(out, inp, 0).unwrap();
    p.set(out, "audiorate", Value::Float(60.0));
    for _ in 0..90 {
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
        p.time.advance(1.0 / 30.0);
    }
    assert!((p.data(out).sample_rate - 48000.0).abs() < 1.0);
    assert!(p.value(inp, "sec") >= 2.0, "sec={}", p.value(inp, "sec"));
    assert!(p.data(inp).channels[0].samples.len() < 8);
}

#[test]
fn ltc_out_holds_the_cue_while_cue_is_on() {
    let mut p = Patch::new();
    let out = p.add("ltcoutCHOP", "ltc1");
    let inp = p.add("ltcinCHOP", "ltc2");
    p.graph.connect(out, inp, 0).unwrap();
    p.set(out, "hour", Value::Int(1));
    p.set(out, "minute", Value::Int(2));
    p.set(out, "second", Value::Int(3));
    p.set(out, "frame", Value::Int(4));
    p.set(out, "cue", Value::Bool(true));
    for _ in 0..90 {
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
        p.time.advance(1.0 / 30.0);
    }
    assert_eq!(p.value(inp, "hour"), 1.0);
    assert_eq!(p.value(inp, "min"), 2.0);
    assert_eq!(p.value(inp, "sec"), 3.0);
    assert_eq!(p.value(inp, "frame"), 4.0);
}

#[test]
fn ltc_out_play_off_does_not_advance() {
    let mut p = Patch::new();
    let out = p.add("ltcoutCHOP", "ltc1");
    let inp = p.add("ltcinCHOP", "ltc2");
    p.graph.connect(out, inp, 0).unwrap();
    p.set(out, "play", Value::Bool(false));
    for _ in 0..90 {
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
        p.time.advance(1.0 / 30.0);
    }
    assert_eq!(p.value(inp, "sec"), 0.0);
    assert_eq!(p.value(inp, "frame"), 0.0);
    assert_eq!(p.value(inp, "total_frames"), 0.0);
}

#[test]
fn ltc_out_timecodeop_reads_chop_channels() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "tc");
    p.set(src, "rows", Value::Int(4));
    for (i, (name, value)) in [
        ("hour", 1.0),
        ("minute", 2.0),
        ("second", 3.0),
        ("frame", 4.0),
    ]
    .into_iter()
    .enumerate()
    {
        p.set(src, &format!("name{i}"), Value::Str(name.into()));
        p.set(src, &format!("value{i}"), Value::Float(value));
    }
    let out = p.add("ltcoutCHOP", "ltc1");
    let inp = p.add("ltcinCHOP", "ltc2");
    p.graph.connect(out, inp, 0).unwrap();
    p.set(out, "playmode", Value::Str("timecodeop".into()));
    p.set(out, "timecodeop", Value::Str("/tc".into()));
    for _ in 0..90 {
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
        p.time.advance(1.0 / 30.0);
    }
    assert_eq!(p.value(inp, "hour"), 1.0);
    assert_eq!(p.value(inp, "min"), 2.0);
    assert_eq!(p.value(inp, "sec"), 3.0);
    assert_eq!(p.value(inp, "frame"), 4.0);
}

#[test]
fn ltc_out_cue_pulse_restarts_from_the_initial_values() {
    let mut p = Patch::new();
    let out = p.add("ltcoutCHOP", "ltc1");
    let inp = p.add("ltcinCHOP", "ltc2");
    p.graph.connect(out, inp, 0).unwrap();
    for _ in 0..150 {
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
        p.time.advance(1.0 / 30.0);
    }
    assert!(p.value(inp, "sec") >= 4.0, "sec={}", p.value(inp, "sec"));
    p.set(out, "cuepulse", Value::Bool(true));
    for _ in 0..60 {
        p.cook
            .cook_frame(&p.graph, &[inp], &p.time, &mut p.host)
            .unwrap();
        p.time.advance(1.0 / 30.0);
    }
    assert!(
        p.value(inp, "sec") < 2.0,
        "sec={} frame={}",
        p.value(inp, "sec"),
        p.value(inp, "frame")
    );
}
