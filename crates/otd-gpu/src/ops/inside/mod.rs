use super::{TopSpec, blend::pack_inside};
use super::composite;

pub const TYPE_NAME: &str = "insideTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Inside",
        "Input 2, kept only where input 1 is opaque.",
        pack_inside,
    )
}
