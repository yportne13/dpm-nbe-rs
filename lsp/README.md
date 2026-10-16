# dpm-nbe-lsp — the language server

> 摘要：把 `dpm-nbe` 的解析器与类型检查器包成一个标准的 LSP 服务端，
> 供 VS Code / Neovim / Emacs / Helix 使用；同一份协议代码也编译成 wasm，
> 跑在 [`web/`](../web/README.md) 的网页版和 VS Code Web 扩展里。

`dpm-nbe-lsp` is a `stdin`/`stdout` [Language Server Protocol][lsp] server for
the language implemented by the `dpm-nbe` crate.  It is deliberately thin: it
owns no type theory of its own and never re-implements the checker.  Everything
it reports comes from `dpm_nbe::parse` / `dpm_nbe::typecheck_named` /
`dpm_nbe::run_named`, so an editor sees exactly what `dpm-nbe nf` would print.

[lsp]: https://microsoft.github.io/language-server-protocol/

## Features

| Feature | Protocol | Notes |
| --- | --- | --- |
| Diagnostics | `textDocument/publishDiagnostics` (push) | Parse errors are reported as `parse-error`, elaboration failures as `type-error`, each with the position the CLI would use (`<uri>:line:col`) and, in `data.rendered`, the CLI's caret diagram. |
| Hover | `textDocument/hover` | The inferred/written type of the name under the cursor (`x : A`, for `let`, λ-pattern and `(x : A) ->` binders), or the documentation of a keyword (`U`, `refl`, `plus`, `(!)`, …). |
| Completion | `textDocument/completion` | The fixed vocabulary plus every name actually in scope at the cursor — de Bruijn scoping, so a `let` binder is *not* offered inside its own type or value. |
| Go to definition | `textDocument/definition` | The binder token, innermost scope first. |
| Outline | `textDocument/documentSymbol` | One symbol per `let`, nested the way the syntax nests them, with the written type as `detail`. |
| Highlighting | `dpm-nbe/tokens` *(extension)* | Token stream from the same lexer the parser uses, so colours and syntax can never disagree. |
| Run | `dpm-nbe/evaluate` *(extension)* | `mode: "nf"` → normal form and type, `mode: "type"` → the type. Same output as the CLI. |
| Pull diagnostics | `dpm-nbe/diagnostics` *(extension)* | For clients that prefer to ask instead of being pushed to. |

Deliberately **not** implemented: formatting, rename, references, semantic
tokens, code actions, signature help, multi-file workspaces.  The language is a
single self-contained program per file, so there is nothing to resolve across
files and no partial parse to invalidate.

## Build and run

```sh
cargo build --release -p dpm-nbe-lsp      # -> target/release/dpm-nbe-lsp

./target/release/dpm-nbe-lsp --version
./target/release/dpm-nbe-lsp --help
./target/release/dpm-nbe-lsp              # speak LSP on stdin/stdout
```

The server is part of the workspace, so `cargo test --workspace` runs its tests
alongside the library's.

## Editor setup

* **VS Code** — use the bundled extension in [`../vscode/`](../vscode/README.md)
  (it starts the binary for you, and has a WebAssembly mode for
  [vscode.dev](https://vscode.dev)).
* **Neovim** (built-in client, no plugin needed):

  ```lua
  vim.filetype.add({ extension = { dpm = 'dpm-nbe' } })
  vim.api.nvim_create_autocmd('FileType', {
    pattern = 'dpm-nbe',
    callback = function(args)
      vim.lsp.start({
        name = 'dpm-nbe',
        cmd = { 'dpm-nbe-lsp' },
        root_dir = vim.fs.dirname(vim.fs.find({ '.git' }, { upward = true })[1]),
      }, { bufnr = args.buf })
    end,
  })
  ```

* **Emacs** (eglot):

  ```elisp
  (add-to-list 'auto-mode-alist '("\\.dpm\\'" . prog-mode))
  (add-to-list 'eglot-server-programs '(prog-mode . ("dpm-nbe-lsp")))
  ```

* **Helix** (`languages.toml`):

  ```toml
  [[language]]
  name = "dpm-nbe"
  scope = "source.dpm-nbe"
  file-types = ["dpm"]
  comment-token = "--"
  language-servers = ["dpm-nbe-lsp"]

  [language-server.dpm-nbe-lsp]
  command = "dpm-nbe-lsp"
  ```

## Architecture

```
lsp/
  src/analysis.rs   lexer + scope scanner + hover/completion/definition/symbols
                    + diagnostics + evaluate   (pure: no I/O, no threads)
  src/protocol.rs   JSON-RPC / LSP: handle_message(&str) -> Vec<String>
  src/stdio.rs      Content-Length framing over real stdin/stdout
  src/main.rs       the dpm-nbe-lsp binary
  tests/lsp.rs      25 integration tests, including the binary over a pipe
```

Three decisions are worth knowing about:

1. **No `tower-lsp` / `lsp-server`.**  The protocol layer is hand-written on
   `serde_json` alone, because the same code has to compile to
   `wasm32-unknown-unknown` (see [`../web/`](../web/README.md)), where the
   thread- and socket-based crates do not work.  `Server::handle_message` takes
   one message and returns the messages to send — the transport is someone
   else's problem.
2. **Everything is re-analysed on every change.**  A file here is a single
   elaboration of a few kilobytes; the parser is bounded (`parse::MAX_DEPTH`)
   and analysis above 200 KiB is refused with a warning instead of hanging the
   editor.  Coalescing is structural: the client debounces, and the server
   answers the latest content it was given.
3. **Analysis is best-effort where the checker is not.**  `analysis.rs` walks
   the token stream with a permissive mirror of the grammar, so hover and
   completion keep working on a file the elaborator rejects.  Diagnostics are
   never produced there — they come from the real parser and elaborator — so a
   wrong guess can not turn into a false error.

Positions are LSP positions (0-based line, 0-based **UTF-16** character); the
parser reports 1-based line and character, and `analysis::LineIndex` is the only
place that converts between the two.

## Tests

```sh
cargo test -p dpm-nbe-lsp           # 25 tests: analysis, protocol, framing, binary
cargo test --workspace              # everything, including the 113 library tests
```

`tests/lsp.rs` covers the lexer (tokens must tile the input exactly), the
coordinate conversions (CRLF, astral characters), scopes on the README program,
diagnostics positions against the CLI's own `<stdin>:1:17:`, the JSON-RPC
surface, ranged and full `didChange`, and a spawned `dpm-nbe-lsp` process driven
over `Content-Length` frames.

## Notes and limits

* `shutdown`/`exit` are honoured; after `exit` the process returns 0.
* A panic inside the checker is caught and reported (`internal-error`
  diagnostic or a JSON-RPC `-32603`), on a 64 MiB stack thread — the same
  arrangement as the CLI.  On `wasm32-unknown-unknown` there is no unwinding,
  so a panic is an instance trap instead; see [`../web/api/CONTRACT.md`](../web/api/CONTRACT.md).
* Diagnostics are recomputed from scratch; there is no incremental re-parse.
* The server keeps documents in memory only, keyed by the URI the client sends.
  It never reads or writes the file system.
