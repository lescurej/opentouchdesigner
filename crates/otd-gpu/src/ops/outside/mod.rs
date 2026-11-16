use super::{TopSpec, blend::pack_outside};
use super::composite;

pub const TYPE_NAME: &str = "outsideTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Outside",
        "Input 2, kept only where input 1 is not.",
        pack_outside,
    )
}
