pub const TYPE_NAME: &str = "lengthCHOP";

use super::ChopSpec;

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Length",
        &["in"],
        "Change the sample count. The sample rate stays the same.",
        super::resample_elemental::params_length,
        super::resample_elemental::cook_length,
    )
}
