//! Device and network CHOPs: audio in, spectrum, MIDI in, OSC in and out,
//! DMX in and out, mouse and keyboard.
//!
//! PLAN.md §4 is explicit that device work does not happen on the cook thread.
//! Audio runs on cpal's callback thread, MIDI on midir's, OSC on its own
//! socket thread; each writes into a small shared buffer, and the cook simply
//! takes whatever is there. Nothing here can block a frame, and a missing
//! device degrades to silence with a message on the node rather than to a
//! failed cook — losing an interface mid-show must not stop the render.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use otd_core::indexmap::IndexMap;
use otd_core::stop_io;
use otd_core::{AudioDeviceList, NodeId, Param};

use crate::data::{CONTROL_RATE, Channel, ChopData, slice_len, slice_times};

mod audio;
mod devices;
mod dmx;
mod midi;
mod osc;
mod panel;

pub use audio::{
    AUDIO_LATENCY_DEFAULT_MS, AUDIO_LATENCY_MAX_MS, AUDIO_LATENCY_MIN_MS, AUDIO_MOVIE,
    clamp_audio_latency_ms,
};
pub(crate) use audio::{
    cook_audio_file, cook_audio_file_out, cook_audio_in, cook_audio_out, cook_reaper_out,
    cook_spectrum, params_audio_file, params_audio_file_out, params_audio_in, params_audio_out,
    params_reaper_out, params_spectrum,
};
pub(crate) use devices::device_name;
pub(crate) use dmx::{cook_dmx_in, cook_dmx_out, params_dmx_in, params_dmx_out};
pub(crate) use midi::{cook_midi_in, cook_midi_out, params_midi_in, params_midi_out};
pub(crate) use osc::{cook_osc_in, cook_osc_out, params_osc_in, params_osc_out};
pub(crate) use panel::{
    cook_keyboard_in, cook_mouse_in, cook_perform_in, params_keyboard_in, params_mouse_in,
    params_perform_in,
};

/// Mouse and keyboard state, pushed in by the editor each frame. The CHOPs
/// read it; `otd-chop` never touches a window system itself.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    /// Normalised to -0.5..0.5 with the origin at the centre of the canvas.
    pub mouse: [f32; 2],
    pub buttons: [bool; 3],
    pub wheel: f32,
    /// Names of the keys currently held, lowercase.
    pub keys: Vec<String>,
}

/// Last perform / panel-root pointer sample (0..1 UV, primary button edges).
#[derive(Clone, Copy, Debug, Default)]
pub struct PerformPanelInput {
    pub u: f32,
    pub v: f32,
    pub lbutton: bool,
    pub click: bool,
    pub release: bool,
    pub wheel: f32,
    pub inside: bool,
}

/// TouchDesigner-style panel values (`focus`, `rollover`, …) for one panel COMP.
#[derive(Clone, Copy, Debug, Default)]
pub struct PanelInteractionSample {
    pub focus: f32,
    pub rollover: f32,
    pub lbutton: f32,
    pub u: f32,
    pub v: f32,
}

/// Per-frame panel interaction, keyed by absolute operator path.
#[derive(Clone, Debug, Default)]
pub struct PanelInteractionStore {
    by_path: HashMap<String, PanelInteractionSample>,
}

impl PanelInteractionStore {
    pub fn clear(&mut self) {
        self.by_path.clear();
    }

    pub fn get(&self, path: &str) -> PanelInteractionSample {
        self.by_path.get(path).copied().unwrap_or_default()
    }

    pub fn set(&mut self, path: String, sample: PanelInteractionSample) {
        self.by_path.insert(path, sample);
    }

    pub fn merge(&mut self, path: String, patch: PanelInteractionSample) {
        let mut cur = self.get(&path);
        if patch.focus > 0.0 {
            cur.focus = patch.focus;
        }
        if patch.rollover > 0.0 {
            cur.rollover = patch.rollover;
            cur.u = patch.u;
            cur.v = patch.v;
        }
        if patch.lbutton > 0.0 {
            cur.lbutton = patch.lbutton;
        }
        self.by_path.insert(path, cur);
    }

    /// Virtual panel-value parameter (`focus`, `rollover`, `lbutton`, `u`, `v`).
    pub fn panel_param(&self, op_path: &str, param: &str) -> Option<f32> {
        let s = self.get(op_path);
        match param {
            "focus" => Some(s.focus),
            "rollover" => Some(s.rollover),
            "lbutton" => Some(s.lbutton),
            "inside" => Some(s.rollover),
            "u" => Some(s.u),
            "v" => Some(s.v),
            _ => None,
        }
    }
}

pub(crate) fn is_panel_widget_op(_op_type: &str) -> bool {
    false
}

#[derive(Default)]
pub struct RecordBuffer {
    pub sample_rate: f64,
    pub channels: Vec<Channel>,
}

#[derive(Default)]
pub struct Io {
    pub input: InputState,
    pub perform_panel: PerformPanelInput,
    pub panel_interaction: PanelInteractionStore,
    pub base_dir: Option<std::path::PathBuf>,
    pub(crate) audio: HashMap<String, audio::AudioIn>,
    pub(crate) audio_out: HashMap<String, audio::AudioOut>,
    pub(crate) files: HashMap<String, audio::AudioFile>,
    pub(crate) audio_file_out: HashMap<String, audio::AudioFileRec>,
    pub(crate) audiomovie: HashMap<String, crate::ops::audiomovie::AudioMovieState>,
    pub(crate) midi: HashMap<String, midi::MidiIn>,
    pub(crate) midi_out: HashMap<String, midi::MidiOut>,
    pub(crate) osc_in: HashMap<String, osc::OscIn>,
    pub(crate) osc_out: HashMap<String, osc::OscOut>,
    pub(crate) reaper_out: HashMap<String, audio::ReaStreamOut>,
    pub(crate) dmx_in: HashMap<String, dmx::DmxIn>,
    pub(crate) dmx: HashMap<String, dmx::DmxOut>,
    pub(crate) spectra: HashMap<String, audio::Spectrum>,
    audio_latency_ms: u32,
    pub perform: crate::perform::PerformFrameStats,
    rt_audio_armed: bool,
    pub chop_files: HashMap<String, (String, std::sync::Arc<crate::data::ChopData>)>,
    pub record_buffers: HashMap<NodeId, RecordBuffer>,
    pub ltc_decode: HashMap<String, crate::timecode::Smpte>,
    ltc_ring: HashMap<String, LtcRing>,
    pub(crate) serial: HashMap<String, crate::ops::serial::SerialLink>,
    pub status: HashMap<String, String>,
}

impl Io {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn note(&mut self, path: &str, message: impl Into<String>) {
        self.status.insert(path.to_string(), message.into());
    }

    pub fn clear_note(&mut self, path: &str) {
        self.status.remove(path);
    }

    pub(crate) fn push_ltc_audio(&mut self, path: &str, samples: &[f32], cap: usize, rate: f64) {
        let ring = self.ltc_ring.entry(path.to_string()).or_default();
        if (ring.rate - rate).abs() > 1.0 && ring.rate > 0.0 {
            ring.samples.clear();
        }
        ring.rate = rate;
        ring.samples.extend_from_slice(samples);
        if ring.samples.len() > cap {
            let excess = ring.samples.len() - cap;
            ring.samples.drain(..excess);
        }
    }

    pub(crate) fn clear_ltc(&mut self, path: &str) {
        self.ltc_ring.remove(path);
        self.ltc_decode.remove(path);
    }

    pub(crate) fn ltc_audio(&self, path: &str) -> Option<&[f32]> {
        self.ltc_ring.get(path).map(|v| v.samples.as_slice())
    }

    pub fn status_of(&self, path: &str) -> Option<&str> {
        self.status.get(path).map(|s| s.as_str())
    }

    pub fn set_audio_rt_armed(&mut self, armed: bool) {
        self.rt_audio_armed = armed;
    }

    /// Closes the frame-paced ReaStream senders of Reaper Outs the audio
    /// worker now streams, so no operator is ever sent twice.
    pub fn release_frame_reaper_outs(&mut self, paths: &[String]) {
        for path in paths {
            self.reaper_out.remove(path);
        }
    }

    pub(crate) fn rt_audio_armed(&self) -> bool {
        self.rt_audio_armed
    }

    pub(crate) fn snapshot_for_rt_cook(&self) -> Io {
        Io {
            input: self.input.clone(),
            perform_panel: self.perform_panel,
            panel_interaction: self.panel_interaction.clone(),
            base_dir: self.base_dir.clone(),
            audio_latency_ms: self.audio_latency_ms,
            rt_audio_armed: self.rt_audio_armed,
            chop_files: self.chop_files.clone(),
            ..Default::default()
        }
    }

    pub fn set_audio_latency_ms(&mut self, ms: u32) {
        let ms = if ms == 0 {
            0
        } else {
            clamp_audio_latency_ms(ms)
        };
        self.audio_latency_ms = ms;
        let jitter = self.reastream_jitter_ms();
        for out in self.reaper_out.values() {
            out.sender.set_jitter_ms(jitter);
        }
    }

    pub fn reastream_jitter_ms(&self) -> u32 {
        self.audio_latency_ms().clamp(30, 300)
    }

    pub fn audio_latency_ms(&self) -> u32 {
        if self.audio_latency_ms == 0 {
            AUDIO_LATENCY_DEFAULT_MS
        } else {
            self.audio_latency_ms
        }
    }

    /// Release every device. Called when a project is closed.
    pub fn reset(&mut self) {
        self.audio.clear();
        self.audio_out.clear();
        self.files.clear();
        self.audio_file_out.clear();
        self.audiomovie.clear();
        self.midi.clear();
        self.midi_out.clear();
        self.osc_in.clear();
        self.osc_out.clear();
        self.reaper_out.clear();
        self.dmx_in.clear();
        self.dmx.clear();
        self.spectra.clear();
        self.chop_files.clear();
        self.record_buffers.clear();
        self.ltc_decode.clear();
        self.ltc_ring.clear();
        self.serial.clear();
        self.status.clear();
    }
}

#[derive(Default)]
struct LtcRing {
    rate: f64,
    samples: Vec<f32>,
}

pub const PERFORM_IN: &str = "performinCHOP";
pub const AUDIO_IN: &str = "audiodeviceinCHOP";
pub const AUDIO_OUT: &str = "audiodeviceoutCHOP";
pub const AUDIO_FILE: &str = "audiofileinCHOP";
pub const AUDIO_FILE_OUT: &str = "audiofileoutCHOP";
pub const FILE_IN: &str = "fileinCHOP";
pub const MIDI_IN: &str = "midiinCHOP";
pub const MIDI_OUT: &str = "midioutCHOP";
pub const OSC_IN: &str = "oscinCHOP";
pub const OSC_OUT: &str = "oscoutCHOP";
pub const REAPER_OUT: &str = "reaperoutCHOP";
pub const SPECTRUM: &str = "audiospectrumCHOP";
pub const AUDIO_FILTER: &str = crate::ops::audiofilter::AUDIO_FILTER;
pub const AUDIO_VST: &str = crate::ops::audiovst::AUDIO_VST;
pub const DMX_IN: &str = "dmxinCHOP";
pub const DMX_OUT: &str = "dmxoutCHOP";

/// Device and network CHOPs that must cook every frame — their effect is
/// outside the graph (speakers, MIDI, OSC, DMX), not a viewer texture.
pub fn is_side_effect_root(op_type: &str) -> bool {
    matches!(
        op_type,
        AUDIO_IN
            | AUDIO_OUT
            | AUDIO_FILE_OUT
            | MIDI_IN
            | MIDI_OUT
            | OSC_IN
            | OSC_OUT
            | REAPER_OUT
            | DMX_IN
            | DMX_OUT
    )
}
pub fn audio_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|ds| ds.filter_map(|d| device_name(&d)).collect())
        .unwrap_or_default()
}

/// Every audio output device the host can see, for the parameter menu.
pub fn audio_output_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.output_devices()
        .map(|ds| ds.filter_map(|d| device_name(&d)).collect())
        .unwrap_or_default()
}

/// Menu entries for an audiodevice CHOP: `(default)` first (stored as an
/// empty value), then host devices, then the current value if it is missing
/// (unplugged hardware).
pub fn audio_device_menu_items(
    list: AudioDeviceList,
    current: &str,
) -> Vec<otd_core::SelectOption> {
    let names = match list {
        AudioDeviceList::Input => audio_input_devices(),
        AudioDeviceList::Output => audio_output_devices(),
        AudioDeviceList::None => Vec::new(),
    };
    let mut items = Vec::new();
    if list != AudioDeviceList::None {
        items.push(otd_core::SelectOption::new("(default)", ""));
    }
    for name in names {
        if name.is_empty() {
            continue;
        }
        items.push(otd_core::SelectOption::same(&name));
    }
    if !current.is_empty() && !items.iter().any(|item| item.value == current) {
        items.push(otd_core::SelectOption::same(current));
    }
    items
}

/// Every MIDI output port, for the parameter menu.
pub fn midi_output_ports() -> Vec<String> {
    let Ok(output) = midir::MidiOutput::new("OpenTouchDesigner") else {
        return Vec::new();
    };
    output
        .ports()
        .iter()
        .filter_map(|p| output.port_name(p).ok())
        .collect()
}

/// Every MIDI input port, for the parameter menu.
pub fn midi_input_ports() -> Vec<String> {
    let Ok(input) = midir::MidiInput::new("OpenTouchDesigner") else {
        return Vec::new();
    };
    input
        .ports()
        .iter()
        .filter_map(|p| input.port_name(p).ok())
        .collect()
}

#[cfg(test)]
#[path = "../tests/io_mod__tests.rs"]
mod tests;
