//! OSC device I/O.


mod in_dev;
mod out_dev;

pub(crate) use in_dev::{cook_osc_in, params_osc_in, OscIn};
#[cfg(test)]
pub(crate) use in_dev::collect_osc;
pub(crate) use out_dev::{cook_osc_out, params_osc_out, OscOut};
