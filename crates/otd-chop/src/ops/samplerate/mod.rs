pub const TYPE_NAME: &str = "samplerateCHOP";

use super::ChopSpec;

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Sample Rate",
        &["in"],
        "Keep the samples and replace the sample-rate metadata.",
        super::resample_elemental::params_samplerate,
        super::resample_elemental::cook_samplerate,
    )
}
