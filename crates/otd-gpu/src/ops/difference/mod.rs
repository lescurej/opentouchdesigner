use super::{TopSpec, blend::pack_difference};
use super::composite;

pub const TYPE_NAME: &str = "differenceTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Difference",
        "The absolute difference between the inputs.",
        pack_difference,
    )
}
