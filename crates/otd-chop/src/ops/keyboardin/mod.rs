pub const TYPE_NAME: &str = "keyboardinCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Keyboard In",
        &[],
        "One channel per named key, 1 while held.",
        crate::io::params_keyboard_in,
        crate::io::cook_keyboard_in,
    )
}
