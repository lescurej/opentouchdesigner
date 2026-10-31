---
name: otd-python
description: Write Python in OpenTouchDesigner — parameter expressions, Script DATs, and the Execute/CHOP Execute/Parameter Execute callback DATs. Use when a parameter needs to be computed, a table generated, or something to happen on a frame, a channel crossing or a parameter change.
---

# Python in OpenTouchDesigner

Python is embedded through PyO3 (`crates/otd-py/src/lib.rs`). There is no TouchDesigner `td` module; use the APIs below.

## Scope (every expression and script)

```python
absTime, time, frame, fps    # absolute seconds, timeline seconds, frame, rate
me                           # OpRef for the operator being cooked
math, random, json           # modules
sin cos tan … clamp fit lerp smoothstep
```

Network helpers:

```python
ch('/lfo1', 'chan1')         # CHOP channel -> float
op('text1')                  # OpRef: .text, table cells, .par.x.val, .run(), UDP/WS on network DATs
par('/blur1', 'size')        # Par object; use .val for the evaluated value
parent('hue')                # custom parameter on enclosing COMP
setpar('/blur1', 'size', 8)  # queue param edit (callbacks only)
mod('text1')                 # execute a Text DAT body as a Python module
```

Reads work only while Python is running (during eval or a callback). Writes (`setpar`, `op().text = …`, `run()`) go through `ScriptQueue` and land on the **next frame** unless `op('x').run(deferred=False)` triggers a **sync cook** during a full engine cook frame.

## Parameter expressions

Set `mode: Expression`. Built-in language is tried first; otherwise Python runs. Expressions must not mutate the graph.

## Script DAT

Set `rows = [[...], ...]` in `source`. Mixed types are stringified per cell.

## Callback DATs

Execute / CHOP Execute / Parameter Execute / Web Server Callbacks DAT — define `def on…` handlers in the Callbacks DAT `source`. `me` is an `OpRef` for the operator that owns the callback.

Web Server (`webserverDAT`): `onHTTPRequest(dat, request, response)`; WebSocket handlers `onWebSocketOpen/Close/Receive*`; `dat.webSocketSendText(client, data)` queues sends applied when the webserver DAT cooks.

UDP Out: `op('udpout1').send('text')` or `sendOSC('/addr', [1, 2.0, 'hi'])` (queued; socket must be active from a prior cook).

## Errors

One-line message on the operator; no traceback in the UI.

## Cost

Compile cache for expressions. Use `otd stats` to profile cooks.
