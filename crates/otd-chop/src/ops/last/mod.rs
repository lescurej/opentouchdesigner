pub const TYPE_NAME: &str = "lastCHOP";

use super::ChopSpec;

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Last",
        &["in"],
        "One sample per channel: the most recent value.",
        super::resample_elemental::params_last,
        super::resample_elemental::cook_last,
    )
}
