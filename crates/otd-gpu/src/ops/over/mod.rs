use super::{TopSpec, blend::pack_over};
use super::composite;

pub const TYPE_NAME: &str = "overTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Over",
        "Input 2 over input 1, by its alpha.",
        pack_over,
    )
}
