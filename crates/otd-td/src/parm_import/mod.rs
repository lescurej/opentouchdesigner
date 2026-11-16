mod apply;
mod mapping;
mod parse;

pub use mapping::parm_target_keys;
pub use parse::parse_parm_block;
pub use apply::apply_parsed_parms;

#[cfg(test)]
pub(crate) use mapping::map_parm_key;
#[cfg(test)]
pub(crate) use parse::parse_parm_line;

#[cfg(test)]
#[path = "../tests/parm_import__tests.rs"]
mod tests;
