# Frame phases

OpenTouchDesigner runs one **frame loop** per displayed tick. Order is fixed so cooks see a stable graph and callbacks cannot corrupt an in-flight pull cook.

## Phase order

1. **Time** — Advance `CookContext` (`frame`, `time`, `abs_time`, `dt`). Scrub/loop handled in the app before cook.
2. **Structural sync** — Replicator sync, clone sync from masters (may add nodes; no cook yet).
3. **Apply queued edits** — Drain `ParamEdit` queue from the **previous** frame (`otd_core::edit::apply`). Includes Python `setpar` and parameter-execute requests.
4. **Pull cook** — Roots: viewer/render/cook, info/param CHOPs and watched ops, perform/window (not canvas visibility). Topological pull with cache; parameter expressions evaluate here (native Rust subset, optional PyO3 fallback).
5. **Callbacks** — CHOP Execute / Parameter Execute / frame DAT hooks run after values for this frame are known; they only **enqueue** edits for phase 3 next frame.
6. **UI** — Inspector and canvas read cooked caches (may call `eval_param` with the same `CookContext`).

Execute DATs are **cook roots** so their network inputs are fresh before Python runs.

## Tests

- `crates/otd-engine/tests/execute.rs` — callback phasing and edit queue.
- `crates/otd-engine/tests/frame_phases.rs` — edit lands on the frame after it was queued.

## COMP local time

Expression variables `time`, `frame`, and `absTime` use **component-local** clocks when evaluated on an operator inside a COMP. Ancestor COMPs may define optional custom parameters `timestart` and `rate` to offset and scale local time.
