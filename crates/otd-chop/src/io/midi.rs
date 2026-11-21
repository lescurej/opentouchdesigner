//! I/O submodule: midi.

use crate::ops::ChopCtx;

use super::*;

// -------------------------------------------------------------- MIDI out

pub(crate) struct MidiOut {
    conn: midir::MidiOutputConnection,
    port: String,
    /// The last 7-bit value sent per channel name. MIDI is an event
    /// protocol, unlike DMX: re-sending an unchanged control every frame
    /// floods the port, so only changes go on the wire.
    sent: HashMap<String, u8>,
}

impl MidiOut {
    fn open(requested: &str) -> Result<MidiOut, String> {
        let output = midir::MidiOutput::new("OpenTouchDesigner").map_err(|e| e.to_string())?;
        let ports = output.ports();
        if ports.is_empty() {
            return Err("no MIDI output ports".into());
        }
        let port = if requested.trim().is_empty() {
            ports[0].clone()
        } else {
            ports
                .iter()
                .find(|p| {
                    output
                        .port_name(p)
                        .map(|n| n.contains(requested))
                        .unwrap_or(false)
                })
                .cloned()
                .ok_or_else(|| format!("no MIDI port matching `{requested}`"))?
        };
        let port_name = output.port_name(&port).unwrap_or_else(|_| "?".into());
        let conn = output
            .connect(&port, "otd-midi-out")
            .map_err(|e| e.to_string())?;
        Ok(MidiOut {
            conn,
            port: port_name,
            sent: HashMap::new(),
        })
    }
}

/// What a channel name means on the wire. The naming mirrors MIDI In, so a
/// patch that listens on `n36` can drive another machine's `n36` by wiring
/// the same channels back out.
pub(crate) enum MidiMapping {
    Note(u8),
    Control(u8),
}

pub(crate) fn midi_mapping(name: &str) -> Option<MidiMapping> {
    if let Some(rest) = name.strip_prefix("cc") {
        return rest
            .parse::<u8>()
            .ok()
            .filter(|n| *n < 128)
            .map(MidiMapping::Control);
    }
    if let Some(rest) = name.strip_prefix('n') {
        return rest
            .parse::<u8>()
            .ok()
            .filter(|n| *n < 128)
            .map(MidiMapping::Note);
    }
    None
}

/// The messages a slice's worth of channel values implies, given what was
/// already sent. Pure, so the wire format is testable with no port open.
pub(crate) fn midi_messages(
    channel: u8,
    values: &[(String, f32)],
    sent: &mut HashMap<String, u8>,
) -> Vec<[u8; 3]> {
    let ch = (channel.saturating_sub(1)) & 0x0f;
    let mut out = Vec::new();
    for (name, value) in values {
        let Some(mapping) = midi_mapping(name) else {
            continue;
        };
        let level = (value.clamp(0.0, 1.0) * 127.0).round() as u8;
        let previous = sent.get(name).copied();
        if previous == Some(level) {
            continue;
        }
        match mapping {
            MidiMapping::Note(note) => {
                // A note is a threshold, not a level: crossing zero is the
                // event. Velocity is the value at the moment it crossed.
                let was_on = previous.unwrap_or(0) > 0;
                let is_on = level > 0;
                match (was_on, is_on) {
                    (false, true) => out.push([0x90 | ch, note, level]),
                    (true, false) => out.push([0x80 | ch, note, 0]),
                    // Still held: a change of pressure is not a new note.
                    _ => {}
                }
            }
            MidiMapping::Control(cc) => out.push([0xb0 | ch, cc, level]),
        }
        sent.insert(name.clone(), level);
    }
    out
}

pub(crate) fn params_midi_out() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "device".into(),
        Param::str("").with_label("Port (blank = first)"),
    );
    m.insert(
        "channel".into(),
        Param::int(1).with_label("Channel").with_range(1.0, 16.0),
    );
    m.insert("active".into(), Param::bool(true).with_label("Active"));
    m
}

pub(crate) fn cook_midi_out(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let get = |k: &str| c.node.param(k).map(|p| p.eval(c.eval));
    let requested = get("device").map(|v| v.as_str()).unwrap_or_default();
    let channel = get("channel").map(|v| v.as_i64()).unwrap_or(1).clamp(1, 16) as u8;
    let active = get("active").map(|v| v.as_bool()).unwrap_or(true);
    let path = c.path.to_string();

    if !active || input.num_channels() == 0 {
        return input;
    }

    let needs_open = match c.io.midi_out.get(&path) {
        Some(m) => !requested.trim().is_empty() && !m.port.contains(&requested),
        None => true,
    };
    if needs_open {
        match MidiOut::open(&requested) {
            Ok(m) => {
                c.io.clear_note(&path);
                c.io.midi_out.insert(path.clone(), m);
            }
            Err(e) => {
                c.io.note(&path, e);
                c.io.midi_out.remove(&path);
            }
        }
    }

    if let Some(out) = c.io.midi_out.get_mut(&path) {
        let values: Vec<(String, f32)> = input
            .channels
            .iter()
            .map(|ch| (ch.name.clone(), ch.last()))
            .collect();
        for message in midi_messages(channel, &values, &mut out.sent) {
            if let Err(e) = out.conn.send(&message) {
                c.io.note(&path, format!("MIDI send: {e}"));
                break;
            }
        }
    }
    input
}

// --------------------------------------------------------------- MIDI in

pub(crate) struct MidiState {
    pub(crate) notes: Box<[f32; 128]>,
    pub(crate) ccs: Box<[f32; 128]>,
    pub(crate) seen_ccs: Vec<u8>,
    pub(crate) last_note: f32,
    pub(crate) velocity: f32,
    pub(crate) pitchbend: f32,
}

impl Default for MidiState {
    fn default() -> Self {
        MidiState {
            notes: Box::new([0.0; 128]),
            ccs: Box::new([0.0; 128]),
            seen_ccs: Vec::new(),
            last_note: 0.0,
            velocity: 0.0,
            pitchbend: 0.0,
        }
    }
}

pub(crate) struct MidiIn {
    _conn: midir::MidiInputConnection<()>,
    state: Arc<Mutex<MidiState>>,
    port: String,
}

impl MidiIn {
    fn open(requested: &str) -> Result<MidiIn, String> {
        let input = midir::MidiInput::new("OpenTouchDesigner").map_err(|e| e.to_string())?;
        let ports = input.ports();
        if ports.is_empty() {
            return Err("no MIDI input ports".into());
        }
        let port = if requested.trim().is_empty() {
            ports[0].clone()
        } else {
            ports
                .iter()
                .find(|p| {
                    input
                        .port_name(p)
                        .map(|n| n.contains(requested))
                        .unwrap_or(false)
                })
                .cloned()
                .ok_or_else(|| format!("no MIDI port matching `{requested}`"))?
        };
        let port_name = input.port_name(&port).unwrap_or_else(|_| "?".into());

        let state = Arc::new(Mutex::new(MidiState::default()));
        let sink = state.clone();
        let conn = input
            .connect(
                &port,
                "otd-midi-in",
                move |_stamp, message, _| {
                    let Ok(mut s) = sink.try_lock() else { return };
                    apply_midi(&mut s, message);
                },
                (),
            )
            .map_err(|e| e.to_string())?;
        Ok(MidiIn {
            _conn: conn,
            state,
            port: port_name,
        })
    }
}

/// Decode the handful of messages a visual patch actually uses.
pub(crate) fn apply_midi(s: &mut MidiState, message: &[u8]) {
    if message.len() < 2 {
        return;
    }
    let status = message[0] & 0xf0;
    let d1 = message[1] & 0x7f;
    let d2 = message.get(2).copied().unwrap_or(0) & 0x7f;
    match status {
        // Note on with zero velocity is a note off, by convention.
        0x90 if d2 > 0 => {
            s.notes[d1 as usize] = d2 as f32 / 127.0;
            s.last_note = d1 as f32;
            s.velocity = d2 as f32 / 127.0;
        }
        0x80 | 0x90 => {
            s.notes[d1 as usize] = 0.0;
            if s.last_note == d1 as f32 {
                s.velocity = 0.0;
            }
        }
        0xb0 => {
            s.ccs[d1 as usize] = d2 as f32 / 127.0;
            if !s.seen_ccs.contains(&d1) && s.seen_ccs.len() < 32 {
                s.seen_ccs.push(d1);
            }
        }
        0xe0 => {
            let raw = ((d2 as i32) << 7 | d1 as i32) - 8192;
            s.pitchbend = raw as f32 / 8192.0;
        }
        _ => {}
    }
}

pub(crate) fn params_midi_in() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "device".into(),
        Param::str("").with_label("Port (blank = first)"),
    );
    m.insert(
        "notes".into(),
        Param::str("").with_label("Notes (e.g. 36 38 42)"),
    );
    m
}

pub(crate) fn cook_midi_in(c: &mut ChopCtx) -> ChopData {
    let requested = c
        .node
        .param("device")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    let wanted_notes: Vec<u8> = c
        .node
        .param("notes")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    let path = c.path.to_string();
    let rate = c.rate();
    let n = slice_len(rate, c.time);

    let needs_open = match c.io.midi.get(&path) {
        Some(m) => !requested.trim().is_empty() && !m.port.contains(&requested),
        None => true,
    };
    if needs_open {
        match MidiIn::open(&requested) {
            Ok(m) => {
                c.io.clear_note(&path);
                c.io.midi.insert(path.clone(), m);
            }
            Err(e) => {
                c.io.note(&path, e);
                c.io.midi.remove(&path);
            }
        }
    }

    let mut channels = Vec::new();
    if let Some(midi) = c.io.midi.get(&path) {
        if let Ok(s) = midi.state.lock() {
            channels.push(Channel::constant("note", s.last_note, n));
            channels.push(Channel::constant("velocity", s.velocity, n));
            channels.push(Channel::constant("pitchbend", s.pitchbend, n));
            for cc in &s.seen_ccs {
                channels.push(Channel::constant(format!("cc{cc}"), s.ccs[*cc as usize], n));
            }
            for note in &wanted_notes {
                channels.push(Channel::constant(
                    format!("n{note}"),
                    s.notes[*note as usize % 128],
                    n,
                ));
            }
        }
    }
    if channels.is_empty() {
        channels.push(Channel::constant("note", 0.0, n));
        channels.push(Channel::constant("velocity", 0.0, n));
        channels.push(Channel::constant("pitchbend", 0.0, n));
        for note in &wanted_notes {
            channels.push(Channel::constant(format!("n{note}"), 0.0, n));
        }
    }
    ChopData::new(channels, rate, true)
}
