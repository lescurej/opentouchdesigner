use otd_core::Connector;

use super::helpers::no_params;
use super::null;
use super::{spec as dat_spec, DatSpec};

pub const TYPE_NAME: &str = "inDAT";

pub fn spec() -> DatSpec {
    let mut spec = dat_spec(
        TYPE_NAME,
        "In",
        &[],
        "A data input on this component's node.",
        no_params,
        null::cook,
    );
    spec.def.connector = Connector::In;
    spec
}
