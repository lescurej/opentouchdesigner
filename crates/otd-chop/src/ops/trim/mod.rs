pub const TYPE_NAME: &str = "trimCHOP";

use super::ChopSpec;

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Trim",
        &["in"],
        "Keep the samples between a start and an end.",
        super::resample_elemental::params_trim,
        super::resample_elemental::cook_trim,
    )
}
