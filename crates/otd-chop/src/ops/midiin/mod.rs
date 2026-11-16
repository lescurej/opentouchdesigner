pub const TYPE_NAME: &str = "midiinCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "MIDI In",
        &[],
        "Notes, velocity, pitch bend and the controls you have moved.",
        crate::io::params_midi_in,
        crate::io::cook_midi_in,
    )
}
