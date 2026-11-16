use otd_core::Connector;

use super::{connector_spec, ChopSpec};

pub const TYPE_NAME: &str = "inCHOP";

pub fn spec() -> ChopSpec {
    connector_spec(
        TYPE_NAME,
        "In",
        &[],
        "A channel input on this component's node.",
        Connector::In,
    )
}