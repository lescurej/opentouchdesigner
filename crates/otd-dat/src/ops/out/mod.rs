use otd_core::Connector;

use super::helpers::no_params;
use super::null;
use super::{spec as dat_spec, DatSpec};

pub const TYPE_NAME: &str = "outDAT";

pub fn spec() -> DatSpec {
    let mut spec = dat_spec(
        TYPE_NAME,
        "Out",
        &["in"],
        "This component's data output.",
        no_params,
        null::cook,
    );
    spec.def.connector = Connector::Out;
    spec
}
