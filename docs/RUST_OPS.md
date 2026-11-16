# Adding operators in Rust

1. Pick the family crate (`otd-gpu`, `otd-chop`, `otd-sop`, `otd-dat`, …).
2. Add a folder `crates/<family>/src/ops/<slug>/mod.rs` (slug = snake_case without the family suffix, e.g. `lfo`, `movie_in`, `constant`).
3. Export from that module:
   - `pub const TYPE_NAME: &str` (or family-specific name constant)
   - `pub(crate) fn params() -> IndexMap<String, Param>` (or `pack` for TOPs)
   - `pub(crate) fn cook(...)` (CHOP/DAT/SOP) or shader path for TOPs
   - `pub fn spec() -> <Family>Spec` using helpers from `ops/mod.rs` (`spec`, `spec_animated`, …)
4. `build.rs` scans `src/ops/*/mod.rs` (skips `shared`) and emits `OUT_DIR/registry_generated.rs`. `folder_specs()` is the full operator table.
5. **CHOP device I/O** — stubs in `ops/<slug>/`, implementation in `crate::io`. See [CHOP_IO.md](CHOP_IO.md).
6. Run `cargo test -p <family-crate>` and `cargo test -p otd-engine` (including `registry_consistency` / docs tests).

Path-ref parameters use `Param::as_path_ref()`. Script parameters use `Param::as_script()`.

Generated operator docs: `otd-engine` `docs` tests and `operator_page()`.

Cross-family movie timecode (TOP `timecodeop` from CHOP channels) lives in `otd-core::movie_timecode_frame_index` so `otd-gpu` does not depend on `otd-chop`.