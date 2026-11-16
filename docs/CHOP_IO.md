# CHOP device I/O operators

TouchDesigner routes MIDI, OSC, DMX, audio devices, and similar inputs through dedicated CHOPs. In OTD the **registry entry** lives under `crates/otd-chop/src/ops/<slug>/`, but the **cook and device state** live in `crates/otd-chop/src/io/`.

## Layout

| Piece | Location |
|-------|----------|
| `TYPE_NAME`, `spec()` | `ops/<slug>/mod.rs` — thin wrapper |
| `params_*`, `cook_*` | `io/audio/`, `io/osc/`, `io/dmx.rs`, … |
| Shared `Io` state | `io/mod.rs` (`audiodeviceinCHOP` reads/writes here) |

Example stub (`dmxout`):

```rust
pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "DMX Out",
        &["in"],
        "…",
        crate::io::params_dmx_out,
        crate::io::cook_dmx_out,
    )
}
```

## Adding a device CHOP

1. Implement `params_*` and `cook_*` next to the transport code in `crate::io`.
2. Add `ops/<slug>/mod.rs` that only calls `spec` / `spec_animated` with those functions.
3. Run `cargo test -p otd-chop`.

Logic-only CHOPs (fan, select, limit, …) keep **params + cook in the op folder** — see [RUST_OPS.md](RUST_OPS.md).

## Why not put cook in the op folder?

Device ops share threads, sockets, and buffers on `Io`. One module owns that state so cooks stay deterministic and the op folders stay registry-sized.