use otd_core::Connector;

use super::helpers::no_params;
use super::null;
use super::{IN, OUT, SopSpec, spec as sop_spec};

pub(super) fn legacy_specs() -> Vec<SopSpec> {
    let mut v = Vec::new();
    let mut in_sop = sop_spec(IN, "In", &[], "A geometry input on this component's node.", no_params, null::cook);
    in_sop.def.connector = Connector::In;
    v.push(in_sop);
    let mut out_sop = sop_spec(OUT, "Out", &["in"], "This component's geometry output.", no_params, null::cook);
    out_sop.def.connector = Connector::Out;
    v.push(out_sop);
    v
}
