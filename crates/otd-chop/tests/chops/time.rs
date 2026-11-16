use super::*;

#[test]
fn a_count_chop_counts_rising_edges() {
    let mut p = Patch::new();
    let lfo = p.add("lfoCHOP", "lfo1");
    let count = p.add("countCHOP", "count1");
    p.graph.connect(lfo, count, 0).unwrap();
    p.set(lfo, "type", Value::Str("square".into()));
    p.set(lfo, "frequency", Value::Float(2.0));
    p.set(count, "threshold", Value::Float(0.0));

    // Two cycles per second for two seconds.
    p.run(count, 120);
    let n = p.value(count, "lfo");
    assert!((3.0..=5.0).contains(&n), "expected about 4 edges, got {n}");
}

#[test]
fn a_trigger_chop_rises_then_releases() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "gate");
    let trig = p.add("triggerCHOP", "trigger1");
    p.graph.connect(c, trig, 0).unwrap();
    p.set(trig, "attack", Value::Float(0.1));
    p.set(trig, "sustain", Value::Float(1.0));
    p.set(trig, "release", Value::Float(0.1));

    p.run(trig, 2);
    assert_eq!(p.value(trig, "chan1"), 0.0, "idle before the gate opens");

    p.set(c, "value0", Value::Float(1.0));
    p.run(trig, 12);
    assert!(p.value(trig, "chan1") > 0.9, "attack reaches full level");

    p.set(c, "value0", Value::Float(0.0));
    p.run(trig, 12);
    assert!(p.value(trig, "chan1") < 0.1, "release returns to zero");
}

#[test]
fn hold_freezes_until_the_trigger_fires() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    let trig = p.add("constantCHOP", "trig");
    let hold = p.add("holdCHOP", "hold1");
    p.graph.connect(src, hold, 0).unwrap();
    p.graph.connect(trig, hold, 1).unwrap();

    p.set(src, "value0", Value::Float(0.25));
    p.set(trig, "value0", Value::Float(1.0));
    p.run(hold, 2);
    assert_eq!(p.value(hold, "chan1"), 0.25, "the rising edge samples");

    // The source moves but the trigger stays high: no new edge, no new value.
    p.set(src, "value0", Value::Float(0.9));
    p.run(hold, 2);
    assert_eq!(p.value(hold, "chan1"), 0.25, "held across a static trigger");

    // Drop and raise it again.
    p.set(trig, "value0", Value::Float(0.0));
    p.run(hold, 1);
    p.set(trig, "value0", Value::Float(1.0));
    p.run(hold, 1);
    assert_eq!(p.value(hold, "chan1"), 0.9, "a new edge samples again");
}

#[test]
fn delay_plays_a_channel_back_later() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let d = p.add("delayCHOP", "delay1");
    p.graph.connect(c, d, 0).unwrap();
    // 5 frames at the control rate.
    p.set(d, "delay", Value::Float(5.0 / 60.0));

    p.set(c, "value0", Value::Float(1.0));
    p.run(d, 1);
    assert_eq!(p.value(d, "chan1"), 0.0, "nothing has arrived yet");

    p.run(d, 5);
    assert_eq!(p.value(d, "chan1"), 1.0, "the value arrives five frames on");
}

#[test]
fn a_trail_chop_keeps_a_window_of_history_oldest_first() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    let trail = p.add("trailCHOP", "trail1");
    p.graph.connect(c, trail, 0).unwrap();
    p.set(trail, "window", Value::Float(1.0));
    p.run(trail, 10);
    p.set(c, "value0", Value::Float(1.0));
    p.run(trail, 5);
    let s = &p.data(trail).channels[0].samples;
    assert_eq!(s.len(), 60, "one second at control rate");
    assert_eq!(s[s.len() - 1], 1.0, "newest last");
    assert_eq!(s[0], 0.0, "oldest first");
    assert_eq!(s.iter().filter(|v| **v == 1.0).count(), 5);
    assert!(
        !p.data(trail).time_sliced,
        "a trail is a buffer, not a slice"
    );
}

#[test]
fn trail_reset_clears_buffered_history() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "value0", Value::Float(1.0));
    let trail = p.add("trailCHOP", "trail1");
    p.graph.connect(c, trail, 0).unwrap();
    p.set(trail, "window", Value::Float(1.0));
    p.run(trail, 10);
    assert!(
        p.data(trail).channels[0].samples.contains(&1.0),
        "trail should hold recent input"
    );
    p.set(trail, "resetpulse", Value::Bool(true));
    p.run(trail, 1);
    assert!(
        p.data(trail).channels[0].samples.iter().all(|v| *v == 0.0),
        "reset pulse clears the ring for that cook"
    );
    p.set(trail, "resetpulse", Value::Bool(false));
    p.run(trail, 3);
    assert_eq!(
        p.data(trail).channels[0]
            .samples
            .iter()
            .filter(|v| **v == 1.0)
            .count(),
        3,
        "after reset, only new cooks are recorded"
    );
}

#[test]
fn trail_reset_held_outputs_zeros_without_recording() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "value0", Value::Float(1.0));
    let trail = p.add("trailCHOP", "trail1");
    p.graph.connect(c, trail, 0).unwrap();
    p.run(trail, 5);
    p.set(trail, "reset", Value::Bool(true));
    p.run(trail, 3);
    assert!(
        p.data(trail).channels[0].samples.iter().all(|v| *v == 0.0),
        "reset held forces zero output"
    );
    p.set(trail, "reset", Value::Bool(false));
    p.run(trail, 1);
    assert_eq!(
        p.data(trail).channels[0]
            .samples
            .iter()
            .filter(|v| **v == 1.0)
            .count(),
        1,
        "recording resumes when reset is released"
    );
}

#[test]
fn shift_moves_samples_later_in_the_buffer() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "pat1");
    p.set(pat, "length", Value::Int(4));
    p.set(pat, "type", Value::Str("ramp".into()));
    let sh = p.add("shiftCHOP", "shift1");
    p.graph.connect(pat, sh, 0).unwrap();
    p.set(sh, "samples", Value::Int(1));
    p.run(sh, 1);
    let s = &p.data(sh).channels[0].samples;
    assert_eq!(s[0], 0.0);
    assert!(s[2] > s[1], "positive shift delays content");
}

#[test]
fn replace_swaps_matching_samples() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "value0", Value::Float(0.5));
    let rep = p.add("replaceCHOP", "rep1");
    p.graph.connect(c, rep, 0).unwrap();
    p.set(rep, "value", Value::Float(0.5));
    p.set(rep, "replace", Value::Float(9.0));
    p.run(rep, 1);
    assert_eq!(p.value(rep, "chan1"), 9.0);
}

#[test]
fn record_chop_length_is_seconds_not_samples() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    let rec = p.add("recordCHOP", "rec");
    p.set(src, "value0", Value::Float(1.0));
    p.graph.connect(src, rec, 0).unwrap();
    p.set(rec, "record", Value::Bool(true));
    // Constant CHOP runs at 60 Hz, so a tenth of a second is 6 samples.
    p.set(rec, "maxlength", Value::Float(0.1));
    p.run(rec, 20);
    assert_eq!(p.data(rec).num_samples(), 6);
}

#[test]
fn record_chop_clear_is_a_rising_edge() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    let rec = p.add("recordCHOP", "rec");
    p.set(src, "value0", Value::Float(1.0));
    p.graph.connect(src, rec, 0).unwrap();
    p.set(rec, "record", Value::Bool(true));
    p.run(rec, 10);
    assert_eq!(p.data(rec).num_samples(), 10);

    p.set(rec, "clear", Value::Bool(true));
    p.run(rec, 1);
    assert_eq!(
        p.data(rec).num_samples(),
        1,
        "turning Clear on drops the buffer, then records this frame"
    );
    p.run(rec, 9);
    assert_eq!(
        p.data(rec).num_samples(),
        10,
        "Clear left on must not wipe the buffer every frame"
    );
}

#[test]
fn record_chop_keeps_history_when_the_input_disappears() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    let rec = p.add("recordCHOP", "rec");
    p.set(src, "value0", Value::Float(0.5));
    p.graph.connect(src, rec, 0).unwrap();
    p.set(rec, "record", Value::Bool(true));
    p.run(rec, 12);
    assert_eq!(p.data(rec).num_samples(), 12);

    p.graph.disconnect(rec, 0).unwrap();
    p.run(rec, 5);
    assert_eq!(p.data(rec).num_samples(), 12);
    assert!((p.data(rec).sample_rate - 60.0).abs() < 1.0);
    assert_eq!(p.value(rec, "chan1"), 0.5);
}

#[test]
fn record_chop_keeps_history_when_channel_count_changes() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    let rec = p.add("recordCHOP", "rec");
    p.set(src, "value0", Value::Float(1.0));
    p.graph.connect(src, rec, 0).unwrap();
    p.set(rec, "record", Value::Bool(true));
    p.run(rec, 10);

    p.set(src, "rows", Value::Int(2));
    p.run(rec, 1);
    p.set(src, "rows", Value::Int(1));
    p.run(rec, 5);

    assert_eq!(p.data(rec).num_channels(), 2);
    assert_eq!(p.data(rec).num_samples(), 16);
    assert_eq!(p.value(rec, "chan1"), 1.0);
}

#[test]
fn record_chop_keeps_its_rate_when_the_input_rate_changes() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    let rec = p.add("recordCHOP", "rec");
    p.set(src, "value0", Value::Float(0.25));
    p.set(src, "rate", Value::Float(48_000.0));
    p.graph.connect(src, rec, 0).unwrap();
    p.set(rec, "record", Value::Bool(true));
    p.run(rec, 2);
    assert_eq!(p.data(rec).num_samples(), 1_600);

    p.set(src, "rate", Value::Float(60.0));
    p.run(rec, 1);
    assert!((p.data(rec).sample_rate - 48_000.0).abs() < 1.0);
    assert_eq!(
        p.data(rec).num_samples(),
        2_400,
        "a 60 Hz frame is resampled into the locked 48 kHz buffer, not used to shrink it"
    );
}

#[test]
fn record_chop_takes_a_static_buffer_once() {
    let mut p = Patch::new();
    let src = p.add("waveCHOP", "wave");
    let rec = p.add("recordCHOP", "rec");
    p.set(src, "length", Value::Int(64));
    p.graph.connect(src, rec, 0).unwrap();
    p.set(rec, "record", Value::Bool(true));
    p.run(rec, 5);
    assert_eq!(
        p.data(rec).num_samples(),
        64,
        "a non-sliced clip is restated every cook and must not be appended again"
    );
}

#[test]
fn record_chop_ignores_a_non_finite_length() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    let rec = p.add("recordCHOP", "rec");
    p.graph.connect(src, rec, 0).unwrap();
    p.set(rec, "record", Value::Bool(true));
    p.run(rec, 10);
    p.set(rec, "maxlength", Value::Float(f64::NAN));
    p.run(rec, 3);
    assert_eq!(p.data(rec).num_samples(), 13);
}
