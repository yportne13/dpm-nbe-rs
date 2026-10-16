# The web IDE's JS ↔ wasm contract

The browser build is not a second implementation: `web/api/dpm-nbe-wasm` is a
thin `wasm-bindgen` binding over the same `dpm-nbe-lsp` crate that the native
binary uses.  The rules below are what the frontend and the smoke test rely on.

## Exports

| Export | Signature | Meaning |
| --- | --- | --- |
| `info()` | `() -> string` | JSON: server name, version, language id/extensions, implemented methods. |
| `LspSession` | `class` | A live language-server instance; `new LspSession()`. |
| `LspSession.send(msg)` | `(string) -> string` | One JSON-RPC message in, a JSON **array** of outgoing messages out. |
| `LspSession.initialized` | getter | Whether `initialize` has been seen. |
| `check(src, mode)` | `(string, string) -> string` | Convenience "run" call: `mode` is `"nf"` or `"type"`. |

`check` returns `{ ok: true, mode, output }` or `{ ok: false, mode, error }`.

## Messages

`send` takes exactly what an editor would put on the wire, e.g.

```json
{ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "processId": null } }
```

and returns the messages the server produced, in order — a response for every
request, and `textDocument/publishDiagnostics` notifications after
`didOpen`/`didChange`/`didSave`/`didClose`.  Every message may also carry its
own `text`, so a single request is self-contained:

```json
{ "jsonrpc": "2.0", "id": 7, "method": "textDocument/hover",
  "params": { "textDocument": { "uri": "file:///a.dpm" }, "text": "U",
              "position": { "line": 0, "character": 0 } } }
```

## Positions

All positions are LSP positions: 0-based lines, 0-based **UTF-16** character
offsets (not bytes, not code points).  The Rust side owns the conversion
(`dpm_nbe_lsp::analysis::LineIndex`); JavaScript must never hand a byte offset
to the module.  For the same reason the frontend maps a textarea selection to a
position by counting UTF-16 units:

```js
const character = lineText.slice(0, columnInCodePoints16).length; // already UTF-16
```

## Errors

* Language errors (parse/type) are **data**: a diagnostic with
  `{ severity, code, source: "dpm-nbe", message, data.rendered }`, never a
  thrown exception.  `data.rendered` is the CLI's caret diagram.
* A trap is **not** recoverable: `wasm32-unknown-unknown` cannot unwind, so a
  Rust panic aborts the instance.  The frontend must catch the thrown error,
  mark the session dead, tell the user to reload, and never call into the dead
  module again.  Everything reachable from the module is bounded instead
  (parser depth budget, `MAX_CHECK_BYTES`, 64 MiB shadow stack).

## Verified by

`node web/tools/smoke.mjs` loads the built module in Node with the wasm bytes
read from disk — no browser needed — and asserts the handshake, diagnostics
(including the exact error position), hover, completion, definition, document
symbols, evaluation, tokens, and that a pathological input returns a diagnostic
instead of trapping.
