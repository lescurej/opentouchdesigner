mod device;

use otd_core::indexmap::IndexMap;
use otd_core::Param;

use super::helpers::with_rate;
use super::{spec_animated, ChopSpec};

pub(crate) use device::SerialLink;

pub const TYPE_NAME: &str = "serialCHOP";

pub(crate) fn params() -> IndexMap<String, Param> {
    with_rate(params! {
        "port" => Param::str("").with_label("Port (blank = first)"),
        "baud" => Param::int(9600).with_label("Baud").with_range(300.0, 921600.0),
        "direction" => Param::menu("in", &["in", "out"]).with_label("Direction"),
        "active" => Param::bool(true).with_label("Active"),
    })
}

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Serial",
        &["in"],
        "Read or write bytes on a serial port.",
        params,
        device::cook_serial,
    )
}
