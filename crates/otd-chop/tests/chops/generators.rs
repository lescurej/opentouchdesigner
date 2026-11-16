use super::*;

#[test]
fn a_constant_chop_produces_named_channels() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "rows", Value::Int(3));
    p.set(c, "name0", Value::Str("chan1".into()));
    p.set(c, "name1", Value::Str("chan2".into()));
    p.set(c, "name2", Value::Str("chan3".into()));
    p.set(c, "value0", Value::Float(0.25));
    p.set(c, "value2", Value::Float(-1.0));
    p.run(c, 1);

    assert_eq!(p.data(c).names(), vec!["chan1", "chan2", "chan3"]);
    assert_eq!(p.value(c, "chan1"), 0.25);
    assert_eq!(p.value(c, "chan3"), -1.0);
}

#[test]
fn noise_defaults_to_six_hundred_samples_off_timeslice() {
    let mut p = Patch::new();
    let n = p.add("noiseCHOP", "noise1");
    p.run(n, 1);
    let d = p.data(n);
    assert!(!d.time_sliced);
    assert_eq!(d.channels[0].samples.len(), 600);
    assert_eq!(d.channels[0].name, "chan1");
}

#[test]
fn noise_channel_names_expand_like_touchdesigner() {
    let mut p = Patch::new();
    let n = p.add("noiseCHOP", "noise1");
    p.set(n, "channelname", Value::Str("chan[1-4]".into()));
    p.run(n, 1);
    let d = p.data(n);
    assert_eq!(d.num_channels(), 4);
    assert_eq!(d.channels[0].name, "chan1");
    assert_eq!(d.channels[3].name, "chan4");
}

#[test]
fn noise_length_is_adjustable() {
    let mut p = Patch::new();
    let n = p.add("noiseCHOP", "noise1");
    p.set(n, "end", Value::Float(128.0));
    p.set(n, "endunit", Value::Str("samples".into()));
    p.run(n, 1);
    assert_eq!(p.data(n).channels[0].samples.len(), 128);
}

#[test]
fn noise_sparse_harmonics_change_the_waveform() {
    let mut p = Patch::new();
    let n = p.add("noiseCHOP", "noise1");
    p.set(n, "type", Value::Str("sparse".into()));
    p.set(n, "end", Value::Float(64.0));
    p.set(n, "endunit", Value::Str("samples".into()));
    p.set(n, "harmon", Value::Int(0));
    p.run(n, 1);
    let flat = p.data(n).channels[0].samples[32];
    p.set(n, "harmon", Value::Int(4));
    p.set(n, "rough", Value::Float(0.3));
    p.set(n, "spread", Value::Float(2.5));
    p.run(n, 1);
    let layered = p.data(n).channels[0].samples[32];
    assert!(
        (flat - layered).abs() > 1e-4,
        "harmonics/spread/rough should affect sparse noise"
    );
}

#[test]
fn an_lfo_traces_the_waveform_it_is_asked_for() {
    let mut p = Patch::new();
    let l = p.add("lfoCHOP", "lfo1");
    p.set(l, "frequency", Value::Float(1.0));
    // One cycle per second at 60 fps: a quarter turn is 15 frames.
    p.run(l, 1);
    assert!(p.value(l, "lfo").abs() < 0.2, "starts near zero");
    p.run(l, 14);
    assert!(p.value(l, "lfo") > 0.9, "quarter cycle is near the peak");
    p.run(l, 15);
    assert!(p.value(l, "lfo").abs() < 0.2, "half cycle is back to zero");
    p.run(l, 15);
    assert!(
        p.value(l, "lfo") < -0.9,
        "three quarters is near the trough"
    );
}

#[test]
fn lfo_channel_name_pattern_expands_range() {
    let mut p = Patch::new();
    let l = p.add("lfoCHOP", "lfo1");
    p.set(l, "name", Value::Str("lfo[1-9]".into()));
    p.run(l, 1);
    let d = p.data(l);
    assert_eq!(d.num_channels(), 9);
    assert_eq!(d.channels[0].name, "lfo1");
    assert_eq!(d.channels[8].name, "lfo9");
}

#[test]
fn time_slicing_follows_real_elapsed_time() {
    let mut p = Patch::new();
    let l = p.add("lfoCHOP", "lfo1");
    p.set(l, "rate", Value::Float(600.0));

    p.time.dt = 1.0 / 60.0;
    p.run(l, 1);
    assert_eq!(p.data(l).num_samples(), 10, "600 Hz over 1/60 s");

    // A frame that took four times as long carries four times the samples,
    // so the waveform does not slow down when the renderer does.
    p.time.dt = 4.0 / 60.0;
    p.cook
        .cook_frame(&p.graph, &[l], &p.time, &mut p.host)
        .unwrap();
    assert_eq!(p.data(l).num_samples(), 40);
}

#[test]
fn lfo_ramp_is_unipolar_zero_to_one() {
    let mut p = Patch::new();
    let l = p.add("lfoCHOP", "lfo1");
    p.set(l, "type", Value::Str("ramp".into()));
    p.set(l, "frequency", Value::Float(1.0));
    p.set(l, "amplitude", Value::Float(1.0));
    p.set(l, "offset", Value::Float(0.0));
    p.set(l, "phase", Value::Float(0.0));
    p.run(l, 1);
    assert!(p.value(l, "lfo").abs() < 0.05, "ramp starts at 0");
    p.run(l, 29);
    assert!((p.value(l, "lfo") - 0.5).abs() < 0.1, "mid cycle near 0.5");
    p.run(l, 30);
    assert!(p.value(l, "lfo") > 0.95, "end of cycle near 1");
}

#[test]
fn lfo_pulse_is_unipolar_while_square_is_bipolar() {
    let mut p = Patch::new();
    let l = p.add("lfoCHOP", "lfo1");
    p.set(l, "frequency", Value::Float(1.0));
    p.set(l, "amplitude", Value::Float(1.0));
    p.set(l, "offset", Value::Float(0.0));
    p.set(l, "pulsewidth", Value::Float(0.25));
    p.set(l, "type", Value::Str("pulse".into()));
    p.run(l, 1);
    assert!(p.value(l, "lfo") > 0.9, "pulse starts high");
    p.run(l, 20);
    assert!(p.value(l, "lfo").abs() < 0.05, "pulse falls to zero");

    p.set(l, "type", Value::Str("square".into()));
    p.run(l, 1);
    assert!(p.value(l, "lfo") > 0.9);
    p.run(l, 30);
    assert!(p.value(l, "lfo") < -0.9, "square goes negative");
}

#[test]
fn a_timer_chop_cycles() {
    let mut p = Patch::new();
    let t = p.add("timerCHOP", "timer1");
    p.set(t, "length", Value::Float(0.5));
    p.run(t, 30);
    assert!(p.value(t, "cycles") >= 1.0, "half-second timer cycled");
    assert!(p.value(t, "timer_fraction") < 0.2, "and wrapped round");
}

#[test]
fn constant_row_pattern_shares_one_value() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "name0", Value::Str("chan[1-4]".into()));
    p.set(c, "value0", Value::Float(0.5));
    p.run(c, 1);
    assert_eq!(p.data(c).num_channels(), 4);
    for name in ["chan1", "chan2", "chan3", "chan4"] {
        assert!((p.value(c, name) - 0.5).abs() < 1e-6);
    }
}

#[test]
fn constant_spreads_chan_1_16_style_names() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "name0", Value::Str("chan[1-16]".into()));
    p.run(c, 1);
    assert_eq!(p.data(c).num_channels(), 16);
    assert_eq!(p.data(c).names()[0], "chan1");
    assert_eq!(p.data(c).names()[15], "chan16");
}

#[test]
fn constant_expands_t_xyz_component_pattern() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "rows", Value::Int(1));
    p.set(c, "name0", Value::Str("t[xyz]".into()));
    p.set(c, "value0", Value::Float(1.0));
    p.run(c, 1);
    assert_eq!(p.data(c).names(), vec!["tx", "ty", "tz"]);
}

#[test]
fn constant_name_field_treats_space_as_a_second_channel() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    p.set(c, "rows", Value::Int(1));
    p.set(c, "name0", Value::Str("left right".into()));
    p.set(c, "value0", Value::Float(1.0));
    p.run(c, 1);
    assert_eq!(p.data(c).names(), vec!["left", "right"]);
}

#[test]
fn pattern_channel_name_expands_bracket_syntax() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "pat1");
    p.set(pat, "name", Value::Str("wave[10-12]".into()));
    p.run(pat, 1);
    assert_eq!(p.data(pat).names(), vec!["wave10", "wave11", "wave12"]);
}

#[test]
fn the_beat_chop_is_a_function_of_time_not_a_counter() {
    // Jumping the clock forward must land on the same phase a run of frames
    // would have reached. A counter would be permanently behind.
    let mut p = Patch::new();
    let b = p.add("beatCHOP", "beat1");
    p.set(b, "tempo", Value::Float(120.0)); // Two beats a second.

    // 0.25 s in: half a beat.
    p.time.advance(0.25);
    p.run(b, 1);
    let after_jump = p.value(b, "ramp");
    assert!(
        (after_jump - 0.5).abs() < 0.05,
        "half a beat should read ~0.5, got {after_jump}"
    );

    let count = p.value(b, "count");
    assert_eq!(count, 0.0, "still in the first beat");
}

#[test]
fn clock_chop_uses_touchdesigner_channel_names_in_units_mode() {
    let mut p = Patch::new();
    let clock = p.add("clockCHOP", "clock1");
    p.set(clock, "output", Value::Str("units".into()));
    p.run(clock, 1);
    let names = p.data(clock).names();
    for ch in [
        "msec", "sec", "min", "hour", "ampm", "wday", "day", "yday", "week", "month", "year",
    ] {
        assert!(names.iter().any(|n| n == ch), "missing channel `{ch}`");
    }
    assert!((0.0..24.0).contains(&p.value(clock, "hour")));
    assert!((0.0..=6.0).contains(&p.value(clock, "wday")));
}

#[test]
fn timecode_chop_emits_frame_from_timeline() {
    let mut p = Patch::new();
    let tc = p.add("timecodeCHOP", "tc1");
    p.set(tc, "ratemenu", Value::Str("30".into()));
    p.run(tc, 1);
    assert_eq!(p.value(tc, "frame"), 0.0);
    for _ in 0..30 {
        p.time.advance(1.0 / 30.0);
        p.cook
            .cook_frame(&p.graph, &[tc], &p.time, &mut p.host)
            .unwrap();
    }
    assert_eq!(p.value(tc, "frame"), 0.0);
    assert_eq!(p.value(tc, "sec"), 1.0);
}
