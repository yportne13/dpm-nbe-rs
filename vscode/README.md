# dpm-nbe — VS Code extension

> 摘要：给 `.dpm` 文件配一个 VS Code 扩展：语言 id `dpm-nbe`、TextMate 语法、
> 以及两种模式的 LSP 客户端 —— 桌面版启动原生 `dpm-nbe-lsp` 可执行文件，
> 网页版（vscode.dev / github.dev）在 Web Worker 里跑同一份 wasm 语言服务器。

## What it adds

* language id `dpm-nbe` for `*.dpm`, with comments (`--`, `{- -}`), brackets and
  a TextMate grammar (`syntaxes/dpm-nbe.tmLanguage.json`),
* diagnostics, hover, completion, go-to-definition and the outline, from
  [`../lsp/`](../lsp/README.md),
* commands: `dpm-nbe: Show normal form and type`, `dpm-nbe: Show type`,
  `dpm-nbe: Restart language server`,
* settings: `dpm-nbe.lsp.path`, `dpm-nbe.lsp.mode`, `dpm-nbe.trace.server`.

## Build

```sh
node vscode/scripts/build.mjs          # wasm + sync + npm install + esbuild + test
node vscode/scripts/build.mjs --skip-wasm   # when the wasm module is already built
cd vscode && npx @vscode/vsce package  # -> dpm-nbe-lsp-0.1.0.vsix
```

`scripts/build.mjs` runs, in order:

1. `node web/build.mjs` — cargo + wasm-bindgen (needed for the web mode),
2. `node vscode/scripts/sync-wasm.mjs` — copies the module into `media/` so the
   browser mode ships the *same* build the web IDE serves,
3. `npm install` in `client/` when `node_modules` is missing,
4. `client/bin/esbuild.mjs` — three bundles:

   | Output | Format | Entry |
   | --- | --- | --- |
   | `client/dist/desktop/extension.js` | CommonJS, `vscode` external | `src/extension.desktop.js` |
   | `client/dist/web/extension.js` | CommonJS | `src/extension.web.js` |
   | `client/dist/web/worker.js` | **ESM** (it `import()`s the glue) | `src/worker.js` |

5. `client/test/transport.test.mjs` — 15 checks against the real wasm module.

## Run it

* **Desktop** — press `F5` in VS Code with this folder open (a `.vscode/launch.json`
  is not required; the *Run and Debug* “Extension” configuration is enough), or
  install the packaged `.vsix`.  The extension looks for the server at
  `dpm-nbe.lsp.path`, then `<workspace>/target/{release,debug}/dpm-nbe-lsp`,
  then `<workspace>/lsp/target/{release,debug}/dpm-nbe-lsp`, then `PATH`.  If it
  cannot find it, the error offers to build it
  (`cargo build --release -p dpm-nbe-lsp`) or to set the path.
* **Web** — the same extension runs on [vscode.dev](https://vscode.dev) and
  github.dev: there is no process to spawn, so the client starts
  `dist/web/worker.js` and the worker fetches `media/dpm_nbe_wasm_bg.wasm`.
  Nothing else is needed — no WASI runtime extension, no COOP/COEP headers.

## How the two modes share code

```
                       ┌────────────────────────────┐
   desktop:  spawn  ──▶ │  dpm-nbe-lsp (native)      │
                       └────────────────────────────┘
                                                            both link the same
                       ┌────────────────────────────┐       dpm-nbe-lsp crate
   web:      Worker ──▶ │  dpm-nbe-wasm (wasm32)     │
                       └────────────────────────────┘
```

* `src/extension.desktop.js` — spawns the binary with `TransportKind.stdio`.
* `src/extension.web.js` — creates the worker and hands it to the **browser**
  `LanguageClient`, whose constructor is `(id, name, clientOptions, worker)`
  (client options first — it differs from the Node one).
* `src/transport.js` — the whole bridge: one in, a list of messages out.  It has
  no Worker globals, which is why the test can run it in Node.
* `src/worker.js` — the shell: buffers messages until the module is ready (an
  `initialize` that arrives during the fetch must not be dropped), then loads
  the glue and calls `startTransport`.
* `src/commands.js` — the shared commands; they use the server's
  `dpm-nbe/evaluate` request, so the result shown in the output channel comes
  from the same code that produces the diagnostics.

## Tests

```sh
node vscode/client/test/transport.test.mjs   # 15 checks: the worker's bridge
node vscode/client/test/jsonrpc.test.mjs     # 14 checks: Microsoft's jsonrpc client against the native binary
node vscode/scripts/build.mjs                # ... plus a full rebuild of everything
```

The bridge is tested against the real wasm module over an in-memory channel:
handshake, `didOpen` diagnostics, hover, the extension request, malformed input
(`-32600`), unknown methods (`-32601`) and unsubscribing.  The second test is the
interoperability check: it spawns `target/release/dpm-nbe-lsp` and drives it with
`vscode-jsonrpc`, i.e. with an implementation that is not ours, so that framing
mistakes or protocol-shape mistakes cannot pass unnoticed.  What is *not* covered
by an automated test is the part that only exists inside VS Code: the desktop
spawn, and `new Worker(...)` in the web extension host.  `.vscode/launch.json`
provides “Run desktop extension” and “Run web extension” configurations.
