use super::{TopSpec, blend::pack_subtract};
use super::composite;

pub const TYPE_NAME: &str = "subtractTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Subtract",
        "Input 2 taken away from input 1.",
        pack_subtract,
    )
}
