pub const TYPE_NAME: &str = "midioutCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "MIDI Out",
        &["in"],
        "Sends `nNN` channels as notes and `ccNN` channels as controls.",
        crate::io::params_midi_out,
        crate::io::cook_midi_out,
    )
}
