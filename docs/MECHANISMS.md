# TD-like mechanisms

## Parameters

- **Constant** and **Expression** only in the UI.
- Expressions are **Python-shaped one-liners** evaluated by the Rust engine when possible; PyO3 is a temporary fallback for syntax outside the subset (`needs_python`).
- Network refs: `ch('op', 'chan')`, `par('op', 'param')`, `op('op')['chan']`, `parent.custom`. Relative op names resolve from the parameter owner’s COMP.
- Right-click a CHOP channel in the parameter panel → **Copy expression** (`op('/path')['chan']`).

## Cook and cache

- Pull cook, per-node cache, `time_dependent` propagation (downstream recooks when fed by animated sources).
- Cook roots: render/cook flags, Execute DATs, **Viewer** (not canvas visibility), Info/Parameter CHOPs, operators they watch, perform/window outputs, Display COMPs.
- Orphan branches off-screen with no viewer and no watcher stay cached (TD-like cost).
- Feedback TOP reads **last frame**; Select TOP reads **this frame** (see GPU tests in `otd-gpu`).

## Typed families and converters

Wires stay within one family except through **converter operators** (e.g. CHOP→TOP, TOP→CHOP). Crossing rules live in `otd_core::crossing` and `OpDef::input_families`. Integration patterns: `crates/otd-engine/tests/converters.rs`.

## CHOP time slicing

CHOPs use `CookContext.dt` for sample counts on time-sliced operators. See `crates/otd-engine/tests/chop_time_slice.rs`.

## Scripts

- **Script / Execute DATs**: Python only (`otd-py`), multiline `source` in the Python editor.
- Callbacks queue edits; they never mutate the graph during cook.

## COMP

- In/Out connectors, clones, replicators, external `.otdc`.
- Custom parameters on COMPs are the component API (`parent.name` in expressions).
- Optional `timestart` / `rate` custom params scale local time for children.

## Viewers (network)

- TOP texture, CHOP scope or **bars** (1 sample), DAT table vs text, SOP **3D preview** (wgpu; Viewer Active: orbit, wheel zoom; right-click: reset, perspective/ortho, shaded/wireframe, **origin & axes**; **W** toggles wireframe), COMP shows internal **Out** TOP when Display is on.
- Flags: Bypass, Display, Cook, Lock, Viewer Active (output pane).
- Interactive 3D gizmos on geo/SOP: **not implemented** (future phase).

## Show output

- **Perform** (fullscreen) and separate **Output** window show the render/viewer chain without the network editor.

## Textport

- **Textport** window (top bar or **Alt+T**): scrollable log (cook errors, callback/setpar issues) and a **Python** input line (`ch`, `op`, `setpar`, statements). Same network context as the root component.

## POP

Not a family yet. Use **particleCHOP** + **geometryCOMP** instancing; create dialog shows a hint when searching “pop”.

## Custom operators

Contributors add Rust ops to family `ops.rs` registries (`docs/RUST_OPS.md`). No dylib/WASM ABI in scope.

## Multi-pass GPU

Feedback and persistent buffers are implemented for Feedback/Cache TOP paths. Broader ISF multi-pass targets are described in `docs/MULTIPASS.md`.
