use otd_core::Connector;

use super::{connector_spec, ChopSpec};

pub const TYPE_NAME: &str = "outCHOP";

pub fn spec() -> ChopSpec {
    connector_spec(
        TYPE_NAME,
        "Out",
        &["in"],
        "This component's channel output.",
        Connector::Out,
    )
}