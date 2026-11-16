pub const TYPE_NAME: &str = "performinCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Perform In",
        &[],
        "Perform panel pointer: u, v, lbutton, click, release, wheel, inside.",
        crate::io::params_perform_in,
        crate::io::cook_perform_in,
    )
}
