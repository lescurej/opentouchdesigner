use super::*;

#[test]
fn audio_oscillator_pitch_is_logarithmic() {
    let mut p = Patch::new();
    let pitch0 = p.add("constantCHOP", "p0");
    p.set(pitch0, "value0", Value::Float(0.0));
    let pitch1 = p.add("constantCHOP", "p1");
    p.set(pitch1, "value0", Value::Float(1.0));
    let osc0 = p.add("audiooscillatorCHOP", "osc0");
    p.set(osc0, "rate", Value::Float(44100.0));
    p.set(osc0, "frequency", Value::Float(440.0));
    p.set(osc0, "type", Value::Str("sine".into()));
    p.graph.connect(pitch0, osc0, 0).unwrap();
    let osc1 = p.add("audiooscillatorCHOP", "osc1");
    p.set(osc1, "rate", Value::Float(44100.0));
    p.set(osc1, "frequency", Value::Float(440.0));
    p.set(osc1, "type", Value::Str("sine".into()));
    p.graph.connect(pitch1, osc1, 0).unwrap();
    for _ in 0..120 {
        p.run(osc0, 1);
        p.run(osc1, 1);
    }
    let z0 = zero_crossings(&p.data(osc0).channels[0].samples);
    let z1 = zero_crossings(&p.data(osc1).channels[0].samples);
    assert!(
        z1 > z0 * 15 / 10,
        "pitch +1 should roughly double crossings: {z0} vs {z1}"
    );
}

#[test]
fn audio_oscillator_units_per_octave_scales_pitch() {
    let mut p = Patch::new();
    let one = p.add("constantCHOP", "one");
    p.set(one, "value0", Value::Float(1.0));
    let twelve = p.add("constantCHOP", "twelve");
    p.set(twelve, "value0", Value::Float(12.0));

    let octave = p.add("audiooscillatorCHOP", "oct");
    p.set(octave, "rate", Value::Float(44100.0));
    p.set(octave, "frequency", Value::Float(440.0));
    p.graph.connect(one, octave, 0).unwrap();

    let semitone = p.add("audiooscillatorCHOP", "semi");
    p.set(semitone, "rate", Value::Float(44100.0));
    p.set(semitone, "frequency", Value::Float(440.0));
    p.set(semitone, "unitsperoctave", Value::Float(12.0));
    p.graph.connect(one, semitone, 0).unwrap();

    let midi = p.add("audiooscillatorCHOP", "midi");
    p.set(midi, "rate", Value::Float(44100.0));
    p.set(midi, "frequency", Value::Float(440.0));
    p.set(midi, "unitsperoctave", Value::Float(12.0));
    p.graph.connect(twelve, midi, 0).unwrap();

    for _ in 0..120 {
        p.run(octave, 1);
        p.run(semitone, 1);
        p.run(midi, 1);
    }
    let up = zero_crossings(&p.data(octave).channels[0].samples);
    let step = zero_crossings(&p.data(semitone).channels[0].samples);
    let same = zero_crossings(&p.data(midi).channels[0].samples);
    assert!(
        up > step * 15 / 10,
        "one unit at 12 per octave is a semitone, not an octave: {up} vs {step}"
    );
    let delta = (same as isize - up as isize).unsigned_abs();
    assert!(
        delta <= 2,
        "12 units at 12 per octave matches one octave: {same} vs {up}"
    );
}

#[test]
fn audio_oscillator_frequency_slider_is_logarithmic() {
    let mut p = Patch::new();
    let osc = p.add("audiooscillatorCHOP", "osc1");
    let param = p.graph.node(osc).param("frequency").unwrap();
    assert!(param.logarithmic);
    assert_eq!(param.log_floor, Some(20.0));

    p.set(osc, "frequency", Value::Float(220.0));
    let text = otd_core::Project::from_graph(&p.graph, &p.reg, 60.0)
        .to_ron()
        .unwrap();
    assert!(
        !text.contains("logarithmic"),
        "the slider scale is part of the operator, not the project"
    );
    let loaded = otd_core::Project::from_ron(&text)
        .unwrap()
        .to_graph(&p.reg)
        .unwrap();
    let id = loaded.find("/osc1").unwrap();
    let param = loaded.node(id).param("frequency").unwrap();
    assert!(param.logarithmic, "loading must keep the log slider");
    assert_eq!(param.log_floor, Some(20.0));
    assert_eq!(param.value, Value::Float(220.0));
}

#[test]
fn audio_oscillator_smooth_pitch_glides_across_the_frame() {
    let mut p = Patch::new();
    let pitch = p.add("constantCHOP", "p");
    p.set(pitch, "value0", Value::Float(0.0));

    let stepped = p.add("audiooscillatorCHOP", "step");
    p.set(stepped, "rate", Value::Float(44100.0));
    p.set(stepped, "frequency", Value::Float(440.0));
    p.graph.connect(pitch, stepped, 0).unwrap();

    let glided = p.add("audiooscillatorCHOP", "glide");
    p.set(glided, "rate", Value::Float(44100.0));
    p.set(glided, "frequency", Value::Float(440.0));
    p.set(glided, "smooth", Value::Bool(true));
    p.graph.connect(pitch, glided, 0).unwrap();

    for _ in 0..30 {
        p.run(stepped, 1);
        p.run(glided, 1);
    }
    p.set(pitch, "value0", Value::Float(1.0));
    p.run(stepped, 1);
    p.run(glided, 1);

    let step = &p.data(stepped).channels[0].samples;
    let glide = &p.data(glided).channels[0].samples;
    let mid = step.len() / 2;
    let step_start = mean_period(&step[..mid]);
    let step_end = mean_period(&step[mid..]);
    let glide_start = mean_period(&glide[..mid]);
    let glide_end = mean_period(&glide[mid..]);
    assert!(
        (step_start - step_end).abs() < 8.0 && step_end < 60.0,
        "smooth off holds the new pitch for the slice: {step_start} then {step_end}"
    );
    assert!(
        glide_start > glide_end + 15.0,
        "smooth on glides across the slice: {glide_start} then {glide_end}"
    );
}

#[test]
fn audio_oscillator_huge_pitch_stays_finite_so_math_add_draws() {
    let mut p = Patch::new();
    let pitch = p.add("constantCHOP", "sensors");
    p.set(pitch, "rows", Value::Int(2));
    p.set(pitch, "name0", Value::Str("4/humidity".into()));
    p.set(pitch, "value0", Value::Float(1013.0));
    p.set(pitch, "name1", Value::Str("tone".into()));
    p.set(pitch, "value1", Value::Float(0.0));
    let osc = p.add("audiooscillatorCHOP", "osc1");
    p.set(osc, "rate", Value::Float(48_000.0));
    p.set(osc, "frequency", Value::Float(440.0));
    p.set(osc, "amplitude", Value::Float(0.5));
    p.graph.connect(pitch, osc, 0).unwrap();
    let math = p.add("mathCHOP", "math1");
    p.graph.connect(osc, math, 0).unwrap();
    p.set(math, "chanop", Value::Str("add".into()));

    p.run(math, 8);

    let osc_data = p.data(osc);
    assert_eq!(
        osc_data.names(),
        vec!["4/humidity".to_string(), "tone".to_string()]
    );
    for ch in &osc_data.channels {
        assert!(
            ch.samples.iter().all(|s| s.is_finite()),
            "{} has a non-finite sample",
            ch.name
        );
    }
    let summed = p.data(math);
    assert_eq!(summed.num_channels(), 1);
    assert_eq!(summed.channels[0].name, "4/humidity");
    assert!(
        summed.channels[0].samples.iter().all(|s| s.is_finite()),
        "combine add should stay on screen, not collapse to ?"
    );
    let peak = summed.channels[0]
        .samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    assert!(
        peak > 0.05,
        "the finite tone should survive the sum, peak={peak}"
    );
}

#[test]
fn audio_oscillator_reset_pulse_is_silent() {
    let mut p = Patch::new();
    let reset = p.add("constantCHOP", "rst");
    p.set(reset, "value0", Value::Float(1.0));
    let osc = p.add("audiooscillatorCHOP", "osc1");
    p.set(osc, "rate", Value::Float(44100.0));
    p.graph.connect(reset, osc, 1).unwrap();
    p.run(osc, 10);
    let peak = p.data(osc).channels[0]
        .samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max);
    assert_eq!(peak, 0.0);
}
