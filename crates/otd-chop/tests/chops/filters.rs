use super::*;

#[test]
fn filter_chop_despike_clamps_a_single_outlier() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "pat1");
    let filt = p.add("filterCHOP", "filt1");
    p.graph.connect(pat, filt, 0).unwrap();
    p.set(pat, "length", Value::Int(5));
    p.set(pat, "chan1", Value::Str("0 0 10 0 0".into()));
    p.set(filt, "type", Value::Str("despike".into()));
    p.set(filt, "width", Value::Float(0.05));
    p.set(filt, "spike", Value::Float(1.0));

    p.run(filt, 1);
    let ch = p.data(filt).channel("chan1").unwrap();
    assert!(
        ch.samples[2].abs() < 2.0,
        "spike should be pulled toward neighbors, got {}",
        ch.samples[2]
    );
}

#[test]
fn filter_highpass_passes_a_step_then_rejects_the_held_value() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    let filt = p.add("filterCHOP", "filt1");
    p.graph.connect(c, filt, 0).unwrap();
    p.set(c, "value0", Value::Float(1.0));
    p.set(filt, "type", Value::Str("highpass".into()));
    p.set(filt, "width", Value::Float(0.2));

    p.run(filt, 1);
    let first = p.value(filt, "chan1");
    assert!(first > 0.5, "the step passes: {first}");
    p.run(filt, 120);
    let settled = p.value(filt, "chan1");
    assert!(settled.abs() < 0.05, "a held value decays out: {settled}");
}

#[test]
fn a_lag_chop_approaches_its_input_instead_of_jumping() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    let lag = p.add("lagCHOP", "lag1");
    p.graph.connect(c, lag, 0).unwrap();
    p.set(c, "value0", Value::Float(1.0));
    p.set(lag, "lagup", Value::Float(0.5));

    p.run(lag, 1);
    let first = p.value(lag, "chan1");
    assert!(first > 0.0 && first < 0.2, "first step is partial: {first}");
    p.run(lag, 120);
    assert!(
        (p.value(lag, "chan1") - 1.0).abs() < 0.05,
        "settles at the target"
    );
}

#[test]
fn a_speed_chop_integrates_its_input() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    let speed = p.add("speedCHOP", "speed1");
    p.graph.connect(c, speed, 0).unwrap();
    p.set(c, "value0", Value::Float(2.0));

    // Two units per second for one second.
    p.run(speed, 60);
    let v = p.value(speed, "chan1");
    assert!((v - 2.0).abs() < 0.05, "expected ~2.0, got {v}");
}

#[test]
fn limit_identity_passes_input_buffers_through() {
    use std::sync::Arc;

    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "pat1");
    let l = p.add("limitCHOP", "limit1");
    p.graph.connect(pat, l, 0).unwrap();
    p.set(pat, "length", Value::Int(64));

    p.run(l, 1);
    assert_eq!(
        Arc::as_ptr(&p.data(pat).channels[0].samples),
        Arc::as_ptr(&p.data(l).channels[0].samples),
        "identity limit should share the input sample buffer"
    );
}

#[test]
fn limit_clamps_wraps_folds_and_quantises() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let l = p.add("limitCHOP", "limit1");
    p.graph.connect(c, l, 0).unwrap();
    p.set(c, "value0", Value::Float(1.75));
    p.set(l, "type", Value::Str("clamp".into()));

    p.run(l, 1);
    assert_eq!(p.value(l, "chan1"), 1.0, "clamp");

    p.set(l, "type", Value::Str("loop".into()));
    p.run(l, 1);
    assert!((p.value(l, "chan1") - 0.75).abs() < 1e-5, "loop wraps");

    p.set(l, "type", Value::Str("zigzag".into()));
    p.run(l, 1);
    // 1.75 folds back down: past the top at 1.0, 0.75 further, so 0.25.
    assert!((p.value(l, "chan1") - 0.25).abs() < 1e-5, "zigzag folds");

    p.set(l, "type", Value::Str("quantise".into()));
    p.set(l, "step", Value::Float(0.5));
    p.run(l, 1);
    assert_eq!(p.value(l, "chan1"), 2.0, "quantise snaps to the step");
}

#[test]
fn limit_normalises_each_channel_into_minus_one_to_one() {
    let mut p = Patch::new();
    let pat = p.add("patternCHOP", "wave");
    let expr = p.add("expressionCHOP", "shape");
    let l = p.add("limitCHOP", "limit1");
    p.graph.connect(pat, expr, 0).unwrap();
    p.graph.connect(expr, l, 0).unwrap();
    p.set(pat, "length", Value::Int(4));
    p.set(pat, "name", Value::Str("low high".into()));
    // Same ramp, opposite sign, so the two channels do not share a range.
    p.set(expr, "expr", Value::Str("v * (1 - 2 * c)".into()));
    p.set(l, "type", Value::Str("off".into()));
    p.set(l, "norm", Value::Bool(true));
    p.set(l, "min", Value::Float(-1.0));
    p.set(l, "max", Value::Float(1.0));
    p.run(l, 1);

    // Ramp samples are 0, 0.25, 0.5, 0.75. Each channel's own ends land on ±1.
    let low = &p.data(l).channel("low").unwrap().samples;
    let high = &p.data(l).channel("high").unwrap().samples;
    assert!(
        (low[0] + 1.0).abs() < 1e-5 && (low[3] - 1.0).abs() < 1e-5,
        "low {low:?}"
    );
    assert!(
        (high[0] - 1.0).abs() < 1e-5 && (high[3] + 1.0).abs() < 1e-5,
        "high {high:?}"
    );

    // A channel that only reaches 0.6 still fills Minimum..Maximum.
    p.set(pat, "length", Value::Int(3));
    p.set(expr, "expr", Value::Str("-1 + i * 0.8".into()));
    p.run(l, 1);
    let partial = &p.data(l).channel("low").unwrap().samples;
    assert!(
        (partial[0] + 1.0).abs() < 1e-5 && (partial[2] - 1.0).abs() < 1e-5,
        "peaks at 0.6 must land on maximum, got {partial:?}"
    );

    // The same samples fill whatever Minimum and Maximum are set to.
    p.set(l, "min", Value::Float(0.0));
    p.set(l, "max", Value::Float(1.0));
    p.run(l, 1);
    let unit = &p.data(l).channel("low").unwrap().samples;
    assert!(
        unit[0].abs() < 1e-5 && (unit[2] - 1.0).abs() < 1e-5,
        "output range follows minimum and maximum, got {unit:?}"
    );

    // A flat channel has no span, so normalise leaves the samples alone.
    p.set(expr, "expr", Value::Str("v".into()));
    p.set(pat, "amplitude", Value::Float(0.0));
    p.set(pat, "offset", Value::Float(3.0));
    p.set(l, "min", Value::Float(-1.0));
    p.set(l, "max", Value::Float(1.0));
    p.set(l, "type", Value::Str("off".into()));
    p.run(l, 1);
    let flat = &p.data(l).channel("low").unwrap().samples;
    assert!(
        flat.iter().all(|v| (v - 3.0).abs() < 1e-5),
        "flat stays at its value, got {flat:?}"
    );

    // One sample has no min and max to compare. The value passes through.
    let c = p.add("constantCHOP", "const1");
    let one = p.add("limitCHOP", "limit_one");
    p.graph.connect(c, one, 0).unwrap();
    p.set(c, "value0", Value::Float(1.75));
    p.set(one, "type", Value::Str("off".into()));
    p.set(one, "norm", Value::Bool(true));
    p.set(one, "min", Value::Float(-1.0));
    p.set(one, "max", Value::Float(1.0));
    p.run(one, 1);
    assert_eq!(p.value(one, "chan1"), 1.75, "the first sample has no span yet");

    p.set(one, "norm", Value::Bool(false));
    p.run(one, 1);
    p.set(one, "norm", Value::Bool(true));
    p.set(c, "value0", Value::Float(-1.0));
    p.run(one, 1);
    p.set(c, "value0", Value::Float(0.6));
    p.run(one, 1);
    assert!(
        (p.value(one, "chan1") - 1.0).abs() < 1e-5,
        "a live peak of 0.6 maps onto maximum, got {}",
        p.value(one, "chan1")
    );
}

#[test]
fn limit_normalises_a_multi_sample_time_slice_per_channel() {
    let mut p = Patch::new();
    let n = p.add("noiseCHOP", "noise1");
    let l = p.add("limitCHOP", "limit1");
    p.graph.connect(n, l, 0).unwrap();
    p.set(n, "timeslice", Value::Bool(true));
    p.set(n, "rate", Value::Float(480.0));
    p.set(n, "type", Value::Str("random".into()));
    p.set(n, "amplitude", Value::Float(4.0));
    p.set(l, "type", Value::Str("off".into()));
    p.set(l, "norm", Value::Bool(true));
    p.set(l, "min", Value::Float(-1.0));
    p.set(l, "max", Value::Float(1.0));
    p.run(l, 1);

    let src = &p.data(n).nth(0).unwrap().samples;
    let out = &p.data(l).nth(0).unwrap().samples;
    assert!(src.len() > 1, "the slice needs more than one sample");
    let (mn, mx) = src
        .iter()
        .copied()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |acc, v| {
            (acc.0.min(v), acc.1.max(v))
        });
    let lo = out.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = out.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (lo + 1.0).abs() < 1e-4 && (hi - 1.0).abs() < 1e-4,
        "slice min {mn} max {mx} maps to {lo}..{hi}, out {out:?}"
    );
}

#[test]
fn slope_is_the_inverse_of_speed() {
    // Integrating then differentiating should give back what went in, which
    // is a stronger statement than either operator's own arithmetic.
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "src");
    let speed = p.add("speedCHOP", "integrate");
    let slope = p.add("slopeCHOP", "differentiate");
    p.graph.connect(c, speed, 0).unwrap();
    p.graph.connect(speed, slope, 0).unwrap();
    p.set(c, "value0", Value::Float(2.0));

    p.run(slope, 10);
    assert!(
        (p.value(slope, "chan1") - 2.0).abs() < 0.05,
        "got {}",
        p.value(slope, "chan1")
    );
}

#[test]
fn a_spring_chop_overshoots_then_settles() {
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    let spring = p.add("springCHOP", "spring1");
    p.graph.connect(c, spring, 0).unwrap();
    p.run(spring, 1);
    assert_eq!(p.value(spring, "chan1"), 0.0, "starts at rest on its input");
    p.set(c, "value0", Value::Float(1.0));
    let mut peak = 0.0f32;
    for _ in 0..240 {
        p.run(spring, 1);
        peak = peak.max(p.value(spring, "chan1"));
    }
    assert!(
        peak > 1.05,
        "underdamped by default, so it overshoots: {peak}"
    );
    assert!((p.value(spring, "chan1") - 1.0).abs() < 0.01, "and settles");
}
