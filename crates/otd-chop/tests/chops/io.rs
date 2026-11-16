use super::*;

#[test]
fn osc_arrives_as_channels() {
    use std::net::UdpSocket;

    let mut p = Patch::new();
    let osc = p.add("oscinCHOP", "oscin1");
    // A high port, to avoid colliding with anything a developer is running.
    const PORT: i64 = 38473;
    p.set(osc, "port", Value::Int(PORT));
    p.run(osc, 1);

    if p.host.engine.status("/oscin1").is_some() {
        eprintln!("skipping: could not bind UDP port {PORT}");
        return;
    }

    let socket = UdpSocket::bind("127.0.0.1:0").expect("bind sender");
    let message = rosc::OscPacket::Message(rosc::OscMessage {
        addr: "/fader/1".into(),
        args: vec![rosc::OscType::Float(0.625)],
    });
    let bytes = rosc::encoder::encode(&message).unwrap();
    socket
        .send_to(&bytes, format!("127.0.0.1:{PORT}"))
        .expect("send");

    // The listener is on its own thread; give it a moment, then cook.
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        p.run(osc, 1);
        if p.data(osc).value("fader/1").is_some() {
            break;
        }
    }
    assert_eq!(
        p.data(osc).value("fader/1"),
        Some(0.625),
        "OSC address became a channel; have {:?}",
        p.data(osc).names()
    );
}

#[test]
fn osc_in_reset_clears_channels_until_off() {
    use std::net::UdpSocket;

    let mut p = Patch::new();
    let osc = p.add("oscinCHOP", "oscin1");
    const PORT: i64 = 38474;
    p.set(osc, "port", Value::Int(PORT));
    p.run(osc, 1);

    if p.host.engine.status("/oscin1").is_some() {
        eprintln!("skipping: could not bind UDP port {PORT}");
        return;
    }

    let socket = UdpSocket::bind("127.0.0.1:0").expect("bind sender");
    let message = rosc::OscPacket::Message(rosc::OscMessage {
        addr: "/level".into(),
        args: vec![rosc::OscType::Float(0.5)],
    });
    let bytes = rosc::encoder::encode(&message).unwrap();
    socket
        .send_to(&bytes, format!("127.0.0.1:{PORT}"))
        .expect("send");

    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        p.run(osc, 1);
        if p.data(osc).value("level").is_some() {
            break;
        }
    }
    assert_eq!(p.data(osc).value("level"), Some(0.5));

    p.set(osc, "reset", Value::Bool(true));
    p.run(osc, 1);
    assert!(
        p.data(osc).value("level").is_none(),
        "reset clears channels; have {:?}",
        p.data(osc).names()
    );

    socket
        .send_to(&bytes, format!("127.0.0.1:{PORT}"))
        .expect("send while reset");
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        p.run(osc, 1);
    }
    assert!(
        p.data(osc).value("level").is_none(),
        "incoming OSC ignored while reset is on"
    );

    p.set(osc, "reset", Value::Bool(false));
    p.run(osc, 1);
    socket
        .send_to(&bytes, format!("127.0.0.1:{PORT}"))
        .expect("send after reset");
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        p.run(osc, 1);
        if p.data(osc).value("level").is_some() {
            break;
        }
    }
    assert_eq!(p.data(osc).value("level"), Some(0.5));
}

#[test]
fn dmx_goes_out_over_the_wire() {
    use std::net::UdpSocket;

    // Receive on the Art-Net port ourselves. If something else on this
    // machine already has it, there is nothing to prove here.
    let Ok(listener) = UdpSocket::bind(("127.0.0.1", otd_chop::dmx::ARTNET_PORT)) else {
        eprintln!("skipping: Art-Net port already in use");
        return;
    };
    listener
        .set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .unwrap();

    let mut p = Patch::new();
    let levels = p.add("constantCHOP", "levels");
    let dmx = p.add("dmxoutCHOP", "dmxout1");
    p.graph.connect(levels, dmx, 0).unwrap();
    p.set(levels, "rows", Value::Int(3));
    p.set(levels, "name0", Value::Str("chan1".into()));
    p.set(levels, "name1", Value::Str("chan2".into()));
    p.set(levels, "name2", Value::Str("chan3".into()));
    p.set(levels, "value0", Value::Float(255.0));
    p.set(levels, "value1", Value::Float(128.0));
    p.set(levels, "value2", Value::Float(0.0));
    p.set(dmx, "address", Value::Str("127.0.0.1".into()));
    p.set(dmx, "universe", Value::Int(2));
    p.run(dmx, 1);

    assert!(
        p.host.engine.status("/dmxout1").is_none(),
        "{:?}",
        p.host.engine.status("/dmxout1")
    );

    let mut buf = [0u8; 1024];
    let (len, _) = listener
        .recv_from(&mut buf)
        .expect("a packet should arrive");
    assert!(len > 20);
    assert_eq!(&buf[0..8], b"Art-Net\0");
    assert_eq!(&buf[14..16], &[2, 0], "universe 2");
    // Channel values are 0..255 in the CHOP and on the wire.
    assert_eq!(buf[18], 255);
    assert_eq!(buf[19], 128);
    assert_eq!(buf[20], 0);

    // A DMX Out passes its input through, so it can sit mid-chain.
    assert_eq!(p.data(dmx).num_channels(), 3);
}

#[test]
fn dmx_arrives_as_channels() {
    use std::net::UdpSocket;

    let mut p = Patch::new();
    let dmx = p.add("dmxinCHOP", "dmxin1");
    p.set(dmx, "universe", Value::Int(2));
    p.set(dmx, "channels", Value::Int(3));
    p.run(dmx, 1);

    if p.host.engine.status("/dmxin1").is_some() {
        eprintln!(
            "skipping: could not bind DMX port ({:?})",
            p.host.engine.status("/dmxin1")
        );
        return;
    }

    let packet = otd_chop::dmx::artnet_packet(2, 0, &[255, 128, 0]);
    let sender = UdpSocket::bind("127.0.0.1:0").expect("bind sender");
    sender
        .send_to(&packet, format!("127.0.0.1:{}", otd_chop::dmx::ARTNET_PORT))
        .expect("send");

    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        p.run(dmx, 1);
        if p.value(dmx, "chan1") > 250.0 {
            break;
        }
    }
    assert_eq!(p.value(dmx, "chan1"), 255.0);
    assert_eq!(p.value(dmx, "chan2"), 128.0);
    assert_eq!(p.value(dmx, "chan3"), 0.0);
}

#[test]
fn a_bad_dmx_address_is_reported_not_fatal() {
    let mut p = Patch::new();
    let levels = p.add("constantCHOP", "levels");
    let dmx = p.add("dmxoutCHOP", "dmxout1");
    p.graph.connect(levels, dmx, 0).unwrap();
    p.set(dmx, "address", Value::Str("not a host".into()));
    p.run(dmx, 1);
    assert!(p.host.engine.status("/dmxout1").is_some());
}

#[test]
fn file_in_loads_och_from_disk() {
    let dir = std::env::temp_dir().join(format!("otd-och-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.och");
    let data = ChopData::new(vec![Channel::new("x", vec![1.0, 2.0, 3.0])], 60.0, false);
    std::fs::write(&path, otd_chop::chopfile::serialize(&data)).unwrap();
    let mut p = Patch::new();
    let f = p.add("fileinCHOP", "file1");
    p.set(f, "file", Value::Str(path.to_string_lossy().into_owned()));
    p.run(f, 1);
    assert_eq!(p.data(f).value("x"), Some(3.0));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn file_in_loads_bclip_from_disk() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wave1.bclip");
    let mut p = Patch::new();
    let f = p.add("fileinCHOP", "file1");
    p.set(f, "file", Value::Str(path.to_string_lossy().into_owned()));
    p.run(f, 1);
    assert_eq!(p.data(f).channels.len(), 2);
    assert_eq!(
        p.data(f).value("chan1"),
        Some(p.data(f).channels[0].samples[249])
    );
    assert!((p.data(f).sample_rate - 25.0).abs() < 0.01);
}

#[test]
fn reaper_out_passes_input_through() {
    let mut p = Patch::new();
    let src = p.add("constantCHOP", "src");
    p.set(src, "rows", Value::Int(1));
    p.set(src, "value0", Value::Float(0.5));
    let out = p.add("reaperoutCHOP", "reaperout1");
    p.set(out, "identifier", Value::Str("default".into()));
    p.graph.connect(src, out, 0).unwrap();
    p.run(out, 1);
    assert_eq!(p.value(out, "chan1"), 0.5);
}
