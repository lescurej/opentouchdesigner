//! OSC device I/O.

mod in_dev;
mod out_dev;

#[cfg(test)]
pub(crate) use in_dev::collect_osc;
pub(crate) use in_dev::{OscIn, cook_osc_in, params_osc_in};
pub(crate) use out_dev::{OscOut, cook_osc_out, params_osc_out};
