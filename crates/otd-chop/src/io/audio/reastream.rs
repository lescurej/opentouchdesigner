use super::super::*;
use crate::ops::ChopCtx;

use crate::oscvoice;

pub(crate) struct ReaStreamOut {
    pub(crate) sender: crate::reastream::ReaStreamSender,
    target: SocketAddr,
    identifier: String,
    last_end: f64,
    content_hash: Option<u64>,
    was_sliced: bool,
}

fn buffer_fingerprint(input: &ChopData) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    let mix = |hash: &mut u64, value: u64| {
        *hash ^= value;
        *hash = hash.wrapping_mul(0x100000001b3);
    };
    mix(&mut hash, input.sample_rate.to_bits());
    mix(&mut hash, input.channels.len() as u64);
    for ch in &input.channels {
        mix(&mut hash, ch.samples.len() as u64);
        for sample in ch.as_slice() {
            mix(&mut hash, u64::from(sample.to_bits()));
        }
    }
    hash
}

fn reaper_enqueue(out: &mut ReaStreamOut, input: &ChopData, end: f64) {
    let decision = oscvoice::push_decision(out.last_end, end);
    if input.time_sliced {
        match decision {
            oscvoice::PushDecision::Skip => return,
            oscvoice::PushDecision::Seek => out.sender.clear(),
            oscvoice::PushDecision::Append if !out.was_sliced => out.sender.clear(),
            oscvoice::PushDecision::Append => {}
        }
        let refs: Vec<&[f32]> = input.channels.iter().map(|ch| ch.samples.as_slice()).collect();
        out.sender.push(&refs);
        out.last_end = end;
        out.content_hash = None;
        out.was_sliced = true;
        return;
    }
    let hash = buffer_fingerprint(input);
    let same = out.content_hash == Some(hash);
    match decision {
        oscvoice::PushDecision::Skip => (),
        oscvoice::PushDecision::Append if same && !out.was_sliced => (),
        oscvoice::PushDecision::Seek | oscvoice::PushDecision::Append => {
            let refs: Vec<&[f32]> = input.channels.iter().map(|ch| ch.samples.as_slice()).collect();
            out.sender.replace(&refs);
            out.last_end = end;
            out.content_hash = Some(hash);
            out.was_sliced = false;
        }
    }
}

pub fn ensure_rt_reaper_sender(
    slot: &mut crate::rt::ReaperStreamSlot,
    net: &str,
    identifier: &str,
    rate: u32,
    channels: usize,
    jitter_ms: u32,
) -> Result<(), String> {
    let key = format!("{net}|{identifier}|{channels}");
    if slot.target != key {
        slot.sender = None;
    }
    if slot.sender.is_none() {
        let target = reastream_target(net)?;
        let nch = channels.clamp(1, crate::rt::MAX_REASTREAM_CHANNELS);
        let sender = reastream_socket().and_then(|socket| {
            crate::reastream::ReaStreamSender::open(
                socket,
                target,
                identifier.to_string(),
                rate,
                nch,
                jitter_ms,
            )
        })?;
        slot.sender = Some(sender);
        slot.target = key;
    }
    if let Some(s) = slot.sender.as_ref() {
        s.set_sample_rate(rate);
        s.set_jitter_ms(jitter_ms);
    }
    Ok(())
}

fn reastream_socket() -> Result<UdpSocket, String> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 0));
    UdpSocket::bind(addr).map_err(|e| format!("ReaStream bind: {e}"))
}

fn reastream_target(host: &str) -> Result<SocketAddr, String> {
    let host = host.trim();
    let host = if host.eq_ignore_ascii_case("localhost") || host.is_empty() {
        "127.0.0.1"
    } else {
        host
    };
    format!("{host}:{}", crate::reastream::DEFAULT_PORT)
        .parse()
        .map_err(|_| format!("bad ReaStream address `{host}`"))
}

pub(crate) fn params_reaper_out() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert("active".into(), Param::bool(true).with_label("Active"));
    m.insert(
        "netaddress".into(),
        Param::str("127.0.0.1").with_label("Network Address"),
    );
    m.insert(
        "identifier".into(),
        Param::str("default").with_label("Identifier"),
    );
    m
}

pub(crate) fn cook_reaper_out(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let path = c.path.to_string();

    if !c.b("active") || input.num_channels() == 0 {
        c.io.reaper_out.remove(&path);
        return input;
    }

    let net = c.s("netaddress");
    let identifier = c.s("identifier");
    let rate_f = if input.sample_rate > 0.0 {
        input.sample_rate
    } else {
        c.time.sample_rate
    };
    let sample_rate = otd_core::canonical_project_sample_rate(rate_f);
    let rate_u32 = sample_rate.round().max(1.0) as u32;

    let target = match reastream_target(&net) {
        Ok(t) => t,
        Err(e) => {
            c.io.note(&path, e);
            return input;
        }
    };

    let needs_open = c
        .io
        .reaper_out
        .get(&path)
        .map(|o| o.target != target || o.identifier != identifier)
        .unwrap_or(true);
    if needs_open {
        c.io.reaper_out.remove(&path);
        let nch = input.num_channels().clamp(1, 64);
        match reastream_socket().and_then(|socket| {
            crate::reastream::ReaStreamSender::open(
                socket,
                target,
                identifier.clone(),
                rate_u32,
                nch,
                c.io.reastream_jitter_ms(),
            )
        }) {
            Ok(sender) => {
                c.io.clear_note(&path);
                c.io.reaper_out.insert(
                    path.clone(),
                    ReaStreamOut {
                        sender,
                        target,
                        identifier: identifier.clone(),
                        last_end: f64::NAN,
                        content_hash: None,
                        was_sliced: false,
                    },
                );
            }
            Err(e) => c.io.note(&path, e),
        }
    }

    if input.num_samples() == 0 {
        return input;
    }

    let jitter_ms = c.io.reastream_jitter_ms();
    if let Some(out) = c.io.reaper_out.get_mut(&path) {
        out.sender.set_sample_rate(rate_u32);
        out.sender.set_jitter_ms(jitter_ms);
        reaper_enqueue(out, &input, c.time.abs_time);
        c.io.clear_note(&path);
    }

    input
}
