use super::{TopSpec, blend::pack_cross};
use super::composite;

pub const TYPE_NAME: &str = "crossTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Cross",
        "Each input survives where the other is not.",
        pack_cross,
    )
}
