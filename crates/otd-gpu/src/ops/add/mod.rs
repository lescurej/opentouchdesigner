use super::{TopSpec, blend::pack_add};
use super::composite;

pub const TYPE_NAME: &str = "addTOP";

pub fn spec() -> TopSpec {
    composite::blend_alias_spec(
        TYPE_NAME,
        "Add",
        "The two inputs summed.",
        pack_add,
    )
}
