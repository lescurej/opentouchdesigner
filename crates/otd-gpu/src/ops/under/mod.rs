use super::{TopSpec, blend::pack_under};
use super::composite;

pub const TYPE_NAME: &str = "underTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Under",
        "Input 1 over input 2 — Over, the other way up.",
        pack_under,
    )
}
