use super::{TopSpec, blend::pack_multiply};
use super::composite;

pub const TYPE_NAME: &str = "multiplyTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Multiply",
        "The two inputs multiplied — darkens.",
        pack_multiply,
    )
}
