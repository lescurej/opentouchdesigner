# Firewheel evaluation (Phase 4)

**Decision: build `otd-audio` in-tree, do not adopt Firewheel yet.**

## Checked

- Real-time graph model, cpal backend, parameter events.
- Licence (MIT/Apache compatible).

## Blockers for OTD

- TD-style CHOP semantics (per-frame `dt` slices, `ChopData` channel names, `time_sliced`) do not map cleanly to Firewheel nodes without a thick adapter layer.
- ReaStream UDP sink and existing `Io` device maps are entangled with `otd-chop::io`.
- Dynamic graph edits must swap plans without freeing on the audio thread; Firewheel supports graph edits but integration would still require a full partitioner.

## Next step

Implement kernels inside `otd-chop` and schedule them via `crates/otd-audio` + `audio_partition.rs`. Revisit Firewheel if the custom plan swap becomes unmaintainable.