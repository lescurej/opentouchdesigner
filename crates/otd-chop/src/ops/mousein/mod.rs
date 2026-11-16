pub const TYPE_NAME: &str = "mouseinCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Mouse In",
        &[],
        "Cursor position and buttons.",
        crate::io::params_mouse_in,
        crate::io::cook_mouse_in,
    )
}
