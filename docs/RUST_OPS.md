# Adding operators in Rust

1. Pick the family crate (`otd-gpu`, `otd-chop`, `otd-sop`, `otd-dat`, …).
2. Add `params_*()` and an `OpDef { type_name, label, family, params, … }` entry in that crate’s `ops.rs` / `defs()`.
3. Register in `otd-engine` `registry()` if not pulled in via family `defs()`.
4. Implement cook in the family engine (`cook_node` match arm or generic path).
5. Run `cargo test -p <family-crate>` and `cargo test -p otd-engine`.

Path-ref parameters use `Param::as_path_ref()`. Script parameters use `Param::as_script()`.

Generated operator docs: `otd-engine` `docs` tests and `operator_page()`.
