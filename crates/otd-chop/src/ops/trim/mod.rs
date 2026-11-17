pub const TYPE_NAME: &str = "trimCHOP";

use super::ChopSpec;

pub fn spec() -> ChopSpec {
    super::spec_animated(
        TYPE_NAME,
        "Trim",
        &["in"],
        "Keep or drop a time range. Relative trims from each end and holds past them.",
        super::resample_elemental::params_trim,
        super::resample_elemental::cook_trim,
    )
}
