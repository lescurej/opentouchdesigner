# Graph ownership spike (Phase 9)

**Decision: engine-owned graph with command inbox (option A) for Phase 10.**

## Compared

| | Engine owns + commands | Persistent `im` graph |
|---|---|---|
| Refactor size | Large one-time pass on `otd-app` edits | Rewrite `otd-core::Graph` |
| Per-frame Python params | Cheap hints, no full clone | Cheap structural sharing |
| Undo/history | Commands replay on engine copy | Snapshots on every undo step |

## Shipped (Phase 10)

- `BackgroundEngine` (`otd-cook`) starts at app launch; UI owns `Graph` while editing.
- Graph snapshot in `CookHint` only when `Graph::audio_topology_generation()` changes.
- `SharedGraph` on `ProjectRuntime` tracks generation for future `Arc` publish.
- `UiEngine::cook_frame` uses `clone_from` (no steal); paused playback cooks on UI thread.
- `GraphCommand` drain calls `publish` while playing so the background thread sees edits.
- Full engine-side command replay and single `EngineClient` loop: **next** if we need one mutex for the whole graph.

## Target (complete option A)

Wire `EngineClient` so the engine thread is always on, publish `FrameSnapshot` + `SharedGraph` to the UI, and apply graph commands on the engine copy before each cook.