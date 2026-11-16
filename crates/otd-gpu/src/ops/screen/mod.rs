use super::{TopSpec, blend::pack_screen};
use super::composite;

pub const TYPE_NAME: &str = "screenTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Screen",
        "Multiply in the negative — lightens.",
        pack_screen,
    )
}
