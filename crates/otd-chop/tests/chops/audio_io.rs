use super::*;

#[test]
fn audiovst_passes_its_main_input_through() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "const1");
    let vst = p.add("audiovstCHOP", "vst1");
    p.graph.connect(src, vst, 0).unwrap();
    p.set(src, "value0", Value::Float(0.75));
    p.run(vst, 2);
    assert!(
        (p.value(vst, "chan1") - 0.75).abs() < 1e-5,
        "stub VST should pass the wired input through"
    );
    assert!(
        p.host.engine.status("/vst1").is_some(),
        "stub VST should surface a status note"
    );
}

#[test]
fn the_audio_spectrum_finds_the_frequency_it_is_given() {
    let mut p = Patch::new();
    let tone = p.add("lfoCHOP", "tone");
    let spectrum = p.add("audiospectrumCHOP", "spectrum1");
    p.graph.connect(tone, spectrum, 0).unwrap();

    // A 4 kHz tone sampled at 48 kHz; timetoraw mode → chan1_m in amplitude.
    p.set(tone, "rate", Value::Float(48000.0));
    p.set(tone, "frequency", Value::Float(4000.0));
    p.set(spectrum, "size", Value::Str("1024".into()));
    p.set(spectrum, "gain", Value::Float(1.0));

    p.run(spectrum, 10);
    let data = p.data(spectrum);
    let mag = data
        .channel("lfo_m")
        .expect("magnitude channel")
        .samples
        .clone();
    let bin_hz = 48000.0_f64 / 1024.0;
    let expected_bin = (4000.0 / bin_hz).round() as usize;
    let loudest = mag
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();
    assert!(
        loudest.abs_diff(expected_bin) <= 2,
        "4 kHz expected near bin {expected_bin}, peak at {loudest}"
    );
    assert!(mag[loudest] > mag[1].max(1e-6) * 4.0, "peak is distinct");
    let phase = data.channel("lfo_p").unwrap().samples[loudest];
    assert!(
        phase.is_finite() && (-180.0..=180.0).contains(&phase),
        "phase in degrees: {phase}"
    );
}

#[test]
fn the_audio_spectrum_visual_mode_still_has_bands() {
    let mut p = Patch::new();
    let tone = p.add("lfoCHOP", "tone");
    let spectrum = p.add("audiospectrumCHOP", "spectrum1");
    p.graph.connect(tone, spectrum, 0).unwrap();
    p.set(tone, "rate", Value::Float(48000.0));
    p.set(tone, "frequency", Value::Float(4000.0));
    p.set(spectrum, "mode", Value::Str("visual".into()));
    p.set(spectrum, "bands", Value::Int(4));
    p.run(spectrum, 10);
    assert!(p.data(spectrum).channel("band1").is_some());
}

#[test]
fn a_missing_audio_device_is_reported_not_fatal() {
    let mut p = Patch::new();
    let audio = p.add("audiodeviceinCHOP", "audio1");
    p.set(audio, "device", Value::Str("no such device at all".into()));
    p.run(audio, 2);

    assert!(
        p.host.engine.status("/audio1").is_some(),
        "the node should carry an explanation"
    );
    // And still produce a channel, so downstream operators keep working.
    assert_eq!(p.value(audio, "chan1"), 0.0);
}

#[test]
fn an_audio_file_follows_the_timeline() {
    // A one-second WAV whose sample value *is* its time in seconds, so the
    // test can ask "what time is the player reading?" by looking at a sample.
    const RATE: u32 = 1000;
    let mut data = Vec::new();
    for i in 0..RATE {
        data.extend_from_slice(&(i as f32 / RATE as f32).to_le_bytes());
    }
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 4).to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());
    wav.extend_from_slice(&32u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);

    let dir = std::env::temp_dir().join(format!("otd-audiofile-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("ramp.wav"), &wav).unwrap();

    let mut p = Patch::new();
    // A relative path, resolved against the project's directory — the same
    // rule external components follow, and what makes a bundle portable.
    p.graph.set_base_dir(Some(dir.clone()));
    let f = p.add("audiofileinCHOP", "file1");
    p.set(f, "file", Value::Str("ramp.wav".into()));

    // Half a second in, the player must be reading the half-second sample:
    // playback is a function of the timeline, not of a private play head.
    p.run(f, 30);
    assert!(
        p.host.engine.status("/file1").is_none(),
        "{:?}",
        p.host.engine.status("/file1")
    );
    assert!(
        (p.value(f, "chan1") - 0.5).abs() < 0.02,
        "at t=0.5 expected ~0.5, got {}",
        p.value(f, "chan1")
    );

    // Looping: at t=1.5 the file has wrapped and reads ~0.5 again.
    p.run(f, 60);
    assert!(
        (p.value(f, "chan1") - 0.5).abs() < 0.02,
        "looped read at t=1.5 gave {}",
        p.value(f, "chan1")
    );

    // `once` instead holds silence after the end.
    p.set(f, "play", Value::Str("once".into()));
    p.run(f, 1);
    assert_eq!(p.value(f, "chan1"), 0.0);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_audio_file_is_reported_not_fatal() {
    let mut p = Patch::new();
    let f = p.add("audiofileinCHOP", "file1");
    p.set(f, "file", Value::Str("no-such-file.wav".into()));
    p.run(f, 2);
    assert!(p.host.engine.status("/file1").is_some());
    assert_eq!(p.value(f, "chan1"), 0.0);
}

#[test]
fn a_missing_audio_output_passes_its_input_through() {
    // Same rule as every output op: losing the device mid-show must not
    // stop the render, and the channels keep flowing to whatever is next.
    let mut p = Patch::new();
    let c = p.add("constantCHOP", "const1");
    let out = p.add("audiodeviceoutCHOP", "audioout1");
    p.graph.connect(c, out, 0).unwrap();
    p.set(c, "value0", Value::Float(0.75));
    p.set(out, "device", Value::Str("no such output device".into()));
    p.run(out, 2);
    assert!(p.host.engine.status("/audioout1").is_some());
    assert_eq!(p.value(out, "chan1"), 0.75);
}

#[test]
fn audio_device_menu_survives_picking_a_device_and_reloading() {
    let mut p = Patch::new();
    let out = p.add("audiodeviceoutCHOP", "audioout1");
    assert_eq!(
        p.graph
            .node(out)
            .param("device")
            .unwrap()
            .audio_device_list(),
        otd_core::AudioDeviceList::Output
    );
    let choices = p
        .graph
        .node(out)
        .param("device")
        .unwrap()
        .menu
        .clone()
        .unwrap();
    assert_eq!(choices[0].option, "(default)");
    assert_eq!(choices[0].value, "");
    assert!(matches!(
        p.graph.node(out).param("device").unwrap().value,
        Value::Select(_)
    ));
    // Callers set the choice as a string. That must not turn the parameter
    // into a text field.
    p.set(out, "device", Value::Str("Speakers".into()));
    assert_eq!(
        p.graph.node(out).param("device").unwrap().value,
        Value::Select("Speakers".into())
    );

    let input = p.add("audiodeviceinCHOP", "audioin1");
    p.set(input, "device", Value::Str("Microphone".into()));

    // An older file stored the choice as a plain string.
    p.graph
        .node_mut(out)
        .params
        .get_mut("device")
        .unwrap()
        .value = Value::Str("Speakers".into());

    let text = otd_core::Project::from_graph(&p.graph, &p.reg, 60.0)
        .to_ron()
        .unwrap();
    let loaded = otd_core::Project::from_ron(&text)
        .unwrap()
        .to_graph(&p.reg)
        .unwrap();

    let out_id = loaded.find("/audioout1").unwrap();
    let device = loaded.node(out_id).param("device").unwrap();
    assert_eq!(
        device.audio_device_list(),
        otd_core::AudioDeviceList::Output
    );
    assert_eq!(device.value, Value::Select("Speakers".into()));
    assert_eq!(device.menu.as_ref().unwrap()[0].option, "(default)");
    assert_eq!(device.menu.as_ref().unwrap()[0].value, "");

    let in_id = loaded.find("/audioin1").unwrap();
    let device = loaded.node(in_id).param("device").unwrap();
    assert_eq!(device.audio_device_list(), otd_core::AudioDeviceList::Input);
    assert_eq!(device.value, Value::Select("Microphone".into()));
}

#[test]
fn audio_file_out_appends_a_time_sliced_input() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    let out = p.add("audiofileoutCHOP", "out1");
    let dir = std::env::temp_dir().join(format!("otd-audio-out-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("take.wav");
    p.set(src, "value0", Value::Float(0.25));
    p.graph.connect(src, out, 0).unwrap();
    p.set(out, "file", Value::Str(path.display().to_string()));
    p.set(out, "record", Value::Bool(true));
    p.run(out, 5);

    assert_eq!(p.value(out, "chan1"), 0.25);
    let wav = otd_chop::wav::load(&path).unwrap();
    assert_eq!(wav.channels, 1);
    assert_eq!(wav.samples.len(), 5);
    assert!(wav.samples.iter().all(|s| (*s - 0.25).abs() < 1e-6));

    p.set(out, "record", Value::Bool(false));
    p.run(out, 2);
    let wav = otd_chop::wav::load(&path).unwrap();
    assert_eq!(
        wav.samples.len(),
        5,
        "stopping record leaves the file as it was"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn audio_file_out_writes_a_whole_buffer_once() {
    let mut p = Patch::new();
    let src = p.add("waveCHOP", "wave1");
    let out = p.add("audiofileoutCHOP", "out1");
    let dir = std::env::temp_dir().join(format!("otd-audio-out-buf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wave.wav");
    p.set(src, "length", Value::Int(8));
    p.graph.connect(src, out, 0).unwrap();
    p.set(out, "file", Value::Str(path.display().to_string()));
    p.set(out, "record", Value::Bool(true));
    p.run(out, 3);

    let wav = otd_chop::wav::load(&path).unwrap();
    assert_eq!(wav.samples.len(), 8);
    let _ = std::fs::remove_dir_all(&dir);
}
