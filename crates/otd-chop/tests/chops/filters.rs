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
    p.set(l, "normscale", Value::Float(1.0));
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

    // Clamp first, then scale whatever survived. 0 and 0.25 both sit on the
    // floor, so the normalised channel is two lows and two highs.
    p.set(expr, "expr", Value::Str("v".into()));
    p.set(l, "type", Value::Str("clamp".into()));
    p.set(l, "min", Value::Float(0.25));
    p.set(l, "max", Value::Float(0.5));
    p.run(l, 1);
    let clamped = &p.data(l).channel("low").unwrap().samples;
    assert!(
        (clamped[0] + 1.0).abs() < 1e-5
            && (clamped[1] + 1.0).abs() < 1e-5
            && (clamped[2] - 1.0).abs() < 1e-5
            && (clamped[3] - 1.0).abs() < 1e-5,
        "clamp then normalise {clamped:?}"
    );

    // Half scale sits halfway between the source and a full -1..1 map.
    p.set(pat, "amplitude", Value::Float(1.0));
    p.set(pat, "offset", Value::Float(0.0));
    p.set(expr, "expr", Value::Str("v * (1 - 2 * c)".into()));
    p.set(l, "type", Value::Str("off".into()));
    p.set(l, "normscale", Value::Float(0.5));
    p.run(l, 1);
    let half = &p.data(l).channel("low").unwrap().samples;
    assert!(
        (half[0] + 0.5).abs() < 1e-5 && (half[3] - 0.875).abs() < 1e-5,
        "half norm scale {half:?}"
    );

    // A flat channel has no span, so normalise leaves the samples alone.
    p.set(pat, "amplitude", Value::Float(0.0));
    p.set(pat, "offset", Value::Float(3.0));
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
    p.set(one, "normscale", Value::Float(1.0));
    p.run(one, 1);
    assert_eq!(p.value(one, "chan1"), 1.75, "a single sample is left alone");
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
    p.set(l, "normscale", Value::Float(1.0));
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
