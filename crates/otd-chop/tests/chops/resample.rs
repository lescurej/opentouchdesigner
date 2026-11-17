use super::*;

#[test]
fn length_chop_changes_the_length_and_keeps_the_shape() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let r = p.add("lengthCHOP", "length1");
    p.graph.connect(pat, r, 0).unwrap();
    p.set(pat, "length", Value::Int(100));
    p.set(r, "length", Value::Int(10));
    p.run(r, 1);

    assert_eq!(p.data(r).num_samples(), 10);
    let src = &p.data(pat).nth(0).unwrap().samples;
    let out = &p.data(r).nth(0).unwrap().samples;
    assert!((out[0] - src[0]).abs() < 1e-5);
    assert!((out[9] - src[99]).abs() < 1e-5);
}

#[test]
fn trim_chop_keeps_samples_in_a_time_range() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let t = p.add("trimCHOP", "trim1");
    p.graph.connect(pat, t, 0).unwrap();
    p.set(pat, "length", Value::Int(100));
    p.set(t, "relative", Value::Str("abs".into()));
    p.set(t, "start", Value::Float(25.0));
    p.set(t, "end", Value::Float(75.0));
    p.set(t, "startunit", Value::Str("samples".into()));
    p.set(t, "endunit", Value::Str("samples".into()));
    p.run(t, 1);

    assert_eq!(p.data(t).num_samples(), 50);
    assert!((p.data(t).sample_rate - 60.0).abs() < 1e-3);
    let out = &p.data(t).nth(0).unwrap().samples;
    let src = &p.data(pat).nth(0).unwrap().samples;
    assert!((out[0] - src[25]).abs() < 1e-5);
    assert!((out[49] - src[74]).abs() < 1e-5);
}

#[test]
fn trim_length_samplerate_replaces_window_new_interval() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let t = p.add("trimCHOP", "trim1");
    let len = p.add("lengthCHOP", "len1");
    let rate = p.add("samplerateCHOP", "rate1");
    p.graph.connect(pat, t, 0).unwrap();
    p.graph.connect(t, len, 0).unwrap();
    p.graph.connect(len, rate, 0).unwrap();
    p.set(pat, "length", Value::Int(100));
    p.set(t, "relative", Value::Str("abs".into()));
    p.set(t, "start", Value::Float(0.0));
    p.set(t, "end", Value::Float(30.0));
    p.set(t, "startunit", Value::Str("samples".into()));
    p.set(t, "endunit", Value::Str("samples".into()));
    p.set(len, "length", Value::Int(24_000));
    p.set(rate, "rate", Value::Float(48_000.0));
    p.run(rate, 1);

    assert_eq!(p.data(t).num_samples(), 30);
    assert_eq!(p.data(rate).num_samples(), 24_000);
    assert!((p.data(rate).sample_rate - 48_000.0).abs() < 1.0);
    let dur = p.data(rate).num_samples() as f64 / p.data(rate).sample_rate;
    assert!((dur - 0.5).abs() < 0.02, "30 samples at 60 Hz → 0.5 s");
}

#[test]
fn length_then_samplerate_preserves_duration_in_seconds() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let len = p.add("lengthCHOP", "len1");
    let rate = p.add("samplerateCHOP", "rate1");
    p.graph.connect(pat, len, 0).unwrap();
    p.graph.connect(len, rate, 0).unwrap();
    p.set(pat, "length", Value::Int(60));
    p.set(pat, "rate", Value::Float(60.0));
    p.set(len, "length", Value::Int(48_000));
    p.set(rate, "rate", Value::Float(48_000.0));
    p.run(rate, 1);

    assert_eq!(p.data(rate).num_samples(), 48_000);
    assert!((p.data(rate).sample_rate - 48_000.0).abs() < 1.0);
    let dur = p.data(rate).num_samples() as f64 / p.data(rate).sample_rate;
    assert!((dur - 1.0).abs() < 0.02, "one second of waveform: {dur}");
}

#[test]
fn trim_relative_defaults_keep_the_whole_channel() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let t = p.add("trimCHOP", "trim1");
    p.graph.connect(pat, t, 0).unwrap();
    p.set(pat, "length", Value::Int(100));
    p.run(t, 1);
    assert_eq!(p.data(t).num_samples(), 100);
    let src = &p.data(pat).nth(0).unwrap().samples;
    let out = &p.data(t).nth(0).unwrap().samples;
    assert!((out[0] - src[0]).abs() < 1e-5);
    assert!((out[99] - src[99]).abs() < 1e-5);
}

#[test]
fn trim_relative_end_is_measured_from_the_end_and_can_hold() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let t = p.add("trimCHOP", "trim1");
    p.graph.connect(pat, t, 0).unwrap();
    p.set(pat, "length", Value::Int(100));
    p.set(t, "start", Value::Float(10.0));
    p.set(t, "end", Value::Float(-10.0));
    p.set(t, "startunit", Value::Str("samples".into()));
    p.set(t, "endunit", Value::Str("samples".into()));
    p.run(t, 1);
    assert_eq!(p.data(t).num_samples(), 80);
    let src = p.data(pat).nth(0).unwrap().copy_samples();
    let out = &p.data(t).nth(0).unwrap().samples;
    assert!((out[0] - src[10]).abs() < 1e-5);
    assert!((out[79] - src[89]).abs() < 1e-5);

    p.set(t, "start", Value::Float(0.0));
    p.set(t, "end", Value::Float(5.0));
    p.run(t, 1);
    assert_eq!(p.data(t).num_samples(), 105);
    let out = &p.data(t).nth(0).unwrap().samples;
    assert!((out[99] - src[99]).abs() < 1e-5);
    assert!((out[104] - src[99]).abs() < 1e-5);
}

#[test]
fn trim_start_past_end_is_one_sample_and_interior_drops_the_middle() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let t = p.add("trimCHOP", "trim1");
    p.graph.connect(pat, t, 0).unwrap();
    p.set(pat, "length", Value::Int(100));
    p.set(t, "relative", Value::Str("abs".into()));
    p.set(t, "start", Value::Float(40.0));
    p.set(t, "end", Value::Float(10.0));
    p.set(t, "startunit", Value::Str("samples".into()));
    p.set(t, "endunit", Value::Str("samples".into()));
    p.run(t, 1);
    assert_eq!(p.data(t).num_samples(), 1);
    let src = p.data(pat).nth(0).unwrap().copy_samples();
    assert!((p.data(t).nth(0).unwrap().samples[0] - src[40]).abs() < 1e-5);

    p.set(t, "discard", Value::Str("interior".into()));
    p.set(t, "end", Value::Float(70.0));
    p.run(t, 1);
    assert_eq!(p.data(t).num_samples(), 70);
    let out = &p.data(t).nth(0).unwrap().samples;
    assert!((out[0] - src[0]).abs() < 1e-5);
    assert!((out[39] - src[39]).abs() < 1e-5);
    assert!((out[40] - src[70]).abs() < 1e-5);
    assert!((out[69] - src[99]).abs() < 1e-5);
}

#[test]
fn trim_current_frame_is_one_sample_at_the_playhead() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let t = p.add("trimCHOP", "trim1");
    p.graph.connect(pat, t, 0).unwrap();
    p.set(pat, "length", Value::Int(100));
    p.set(t, "relative", Value::Str("cur".into()));
    p.run(t, 31);
    assert_eq!(p.data(t).num_samples(), 1);
    let src = &p.data(pat).nth(0).unwrap().samples;
    assert!((p.data(t).nth(0).unwrap().samples[0] - src[30]).abs() < 1e-4);
}

#[test]
fn last_then_samplerate_is_one_sample_at_audio_rate() {
    let mut p = Patch::new();
    let wave = p.add("waveCHOP", "wave1");
    let last = p.add("lastCHOP", "last1");
    let rate = p.add("samplerateCHOP", "rate1");
    p.graph.connect(wave, last, 0).unwrap();
    p.graph.connect(last, rate, 0).unwrap();
    p.set(wave, "length", Value::Int(3600));
    p.set(wave, "rate", Value::Float(60.0));
    p.set(rate, "rate", Value::Float(48_000.0));
    p.run(rate, 1);

    assert_eq!(p.data(rate).num_samples(), 1);
    assert!((p.data(rate).sample_rate - 48_000.0).abs() < 1.0);
    let src_last = p
        .data(wave)
        .nth(0)
        .unwrap()
        .samples
        .last()
        .copied()
        .unwrap();
    assert!((p.value(rate, "chan1") - src_last).abs() < 1e-5);
}
