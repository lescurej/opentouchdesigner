# Adding operators in Rust

1. Pick the family crate (`otd-gpu`, `otd-chop`, `otd-sop`, `otd-dat`, …).
2. Add `params_*()` and `pack_*()` in `otd-gpu/src/ops/definitions.rs` (TOP), or `params_*` / `cook_*` in `otd-chop/src/ops/operators.rs` or `otd-dat/src/ops/operators.rs` (other families).
3. Register the spec in the family `table.rs` (or `otd-gpu` `extra.rs` for macro TOPs).
4. Implement cook in the family engine (`cook_node` match arm or generic path).
5. Run `cargo test -p <family-crate>` and `cargo test -p otd-engine`.

Path-ref parameters use `Param::as_path_ref()`. Script parameters use `Param::as_script()`.

Generated operator docs: `otd-engine` `docs` tests and `operator_page()`.
