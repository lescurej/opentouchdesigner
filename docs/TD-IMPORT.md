# TouchDesigner import (`.toe` / `.tox`)

OpenTouchDesigner does not use binary TD projects as its native format (that is
`.otd`), but it can **decode** them and report **which operator types are missing**
compared to the OTD registry. That is the first step toward real migration.

## Decode path

Same idea as [WebToe](https://webtoe.openaudiovisual.com/docs/toe-format/):

1. **Native decode** — TEA + zlib container in `crates/otd-td` (see `container.rs`).
2. **`toeexpand`** — if native decode fails or fails sanity checks (needs a
   TouchDesigner install; on macOS: `TouchDesigner.app/Contents/MacOS/toeexpand`).
3. **Message** — if both fail, the app shows a dialog / CLI prints notes on what
   to do next (install TD, use WebToe, etc.).

Provenance for the binary format is documented by the WebToe project; OTD contains
no Derivative code.

## CLI

```bash
otd td report path/to/file.tox
```

Prints a text report: decode source, node counts, and TD types not in OTD
(`FAMILY:typename`, e.g. `TOP:geotext`).

## Editor

Drag and drop a `.toe` or `.tox` anywhere on the window. A dialog lists the parity
report and an **Import network** button builds the expanded tree into the current
component (nodes, wires, basic parameters, and `.data` sidecar paths).

Opening a TD file from Finder (`open -a OpenTouchDesigner file.tox`) runs the same
report.

## Roadmap

| Step | Status |
|------|--------|
| Expand container (native + toeexpand) | Done |
| Parity report vs `OpRegistry` | Done |
| Partial graph → current network (known ops + placeholders) | Done (v1) |
| Parameters, wires, expressions | Planned |
| Chunk recovery for huge `.toe` without toeexpand | Planned |

When you add a TD operator to OTD, drop the same `.tox` again — the missing list
should shrink.

## Tests

`cargo test -p otd-td` includes an optional check against `/tmp/protomarquee.tox`
if you copy a file there and run `toeexpand` once (compares native decode file
count to `toeexpand` output).
