use super::*;

#[test]
fn a_low_pass_audio_filter_kills_a_tone_above_cutoff() {
    let mut p = Patch::new();
    let osc = p.add("audiooscillatorCHOP", "osc1");
    let flt = p.add("audiofilterCHOP", "flt1");
    p.graph.connect(osc, flt, 0).unwrap();

    p.set(osc, "rate", Value::Float(48_000.0));
    p.set(osc, "frequency", Value::Float(4000.0));
    p.set(osc, "amplitude", Value::Float(1.0));
    p.set(flt, "filter", Value::Str("lowpass".into()));
    p.set(flt, "units", Value::Str("frequency".into()));
    p.set(flt, "cutofffrequency", Value::Float(200.0));
    p.set(flt, "rolloff", Value::Str("24".into()));
    p.set(flt, "drywet", Value::Float(1.0));

    p.run(flt, 120);
    let rms = rms_channel(&p, flt, "chan1");
    assert!(
        rms < 0.05,
        "4 kHz through 200 Hz low-pass should be quiet, rms={rms}"
    );

    p.set(flt, "drywet", Value::Float(0.0));
    p.run(flt, 8);
    let dry_rms = rms_channel(&p, flt, "chan1");
    assert!(
        dry_rms > 0.3,
        "dry mix should pass the tone through, rms={dry_rms}"
    );
}

#[test]
fn a_low_cutoff_audio_filter_stays_finite_at_48k() {
    let mut p = Patch::new();
    let osc = p.add("audiooscillatorCHOP", "osc1");
    let flt = p.add("audiofilterCHOP", "flt1");
    p.graph.connect(osc, flt, 0).unwrap();
    p.set(osc, "rate", Value::Float(48_000.0));
    p.set(osc, "frequency", Value::Float(220.0));
    p.set(osc, "amplitude", Value::Float(1.0));
    p.set(flt, "filter", Value::Str("lowpass".into()));
    p.set(flt, "units", Value::Str("frequency".into()));
    p.set(flt, "cutofffrequency", Value::Float(1.0));
    p.set(flt, "resonance", Value::Float(4.0));
    p.set(flt, "rolloff", Value::Str("24".into()));
    p.set(flt, "drywet", Value::Float(1.0));
    for _ in 0..60 {
        p.run(flt, 1);
        let samples = p.data(flt).channel("chan1").expect("chan1").samples.clone();
        assert!(
            samples.iter().all(|s| s.is_finite()),
            "low cutoff must not blow up the biquad state"
        );
        assert!(
            samples.iter().all(|s| s.abs() <= 8.5),
            "low cutoff must stay within the state limiter"
        );
    }
}

#[test]
fn a_low_cutoff_audio_filter_survives_max_resonance() {
    let mut p = Patch::new();
    let osc = p.add("audiooscillatorCHOP", "osc1");
    let flt = p.add("audiofilterCHOP", "flt1");
    p.graph.connect(osc, flt, 0).unwrap();
    p.set(osc, "rate", Value::Float(48_000.0));
    p.set(osc, "frequency", Value::Float(110.0));
    p.set(osc, "amplitude", Value::Float(1.0));
    p.set(flt, "filter", Value::Str("lowpass".into()));
    p.set(flt, "units", Value::Str("frequency".into()));
    p.set(flt, "cutofffrequency", Value::Float(1.0));
    p.set(flt, "resonance", Value::Float(10.0));
    p.set(flt, "rolloff", Value::Str("24".into()));
    p.set(flt, "drywet", Value::Float(1.0));
    for _ in 0..120 {
        p.run(flt, 1);
        let samples = p.data(flt).channel("chan1").expect("chan1").samples.clone();
        assert!(
            samples.iter().all(|s| s.is_finite() && s.abs() <= 8.5),
            "max resonance at 1 Hz must remain stable"
        );
    }
}

#[test]
fn an_audio_filter_accepts_a_control_rate_chop() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "const1");
    let flt = p.add("audiofilterCHOP", "flt1");
    p.graph.connect(src, flt, 0).unwrap();
    p.set(src, "value0", Value::Float(0.5));

    // The Sample Rate slider bottoms out at 1 Hz. That used to panic in the
    // cutoff clamp (`min = 1.0, max = 0.49`).
    for rate in [1.0, 60.0] {
        p.set(src, "rate", Value::Float(rate));
        p.run(flt, 4);
        let samples = p.data(flt).channel("chan1").expect("chan1").samples.clone();
        assert!(!samples.is_empty(), "rate {rate} produced no samples");
        assert!(
            samples.iter().all(|s| s.is_finite()),
            "rate {rate} produced non-finite samples {samples:?}"
        );
    }
}
