# dpm-nbe — Rust port

A Rust re-implementation of the Haskell project
[`KonjacSource/dpm-nbe`](../Haskell/dpm-nbe) (**dependent pattern matching
under a normalization-by-evaluation setup**).

* ~2.4k lines of implementation, **no dependencies** (standard library only).
* Library + `dpm-nbe` command-line tool.
* 113 unit tests + 7 black-box tests (116 checks).
* [`NOTES.md`](NOTES.md) documents the algorithm and every deliberate
  difference from the Haskell original.

On top of the library, the repository also carries a language server and an
online IDE, so the language can be edited with diagnostics, hover, completion
and go-to-definition:

* [`lsp/`](lsp/README.md) — `dpm-nbe-lsp`, a stdio LSP server (its only
  dependency is `serde_json`), plus 25 integration tests.
* [`web/`](web/README.md) — the same server compiled to WebAssembly and served
  as a zero-dependency web IDE; the page speaks real JSON-RPC/LSP to it.
  Hosted at <https://yportne13.github.io/dpm-nbe-rs/> (GitHub Pages, built by
  [`.github/workflows/pages.yml`](.github/workflows/pages.yml)).
* [`vscode/`](vscode/README.md) — a VS Code extension that starts the native
  binary on the desktop and the wasm module in the browser.

## Build and run

```sh
cargo build --release
cargo test

# read a program from stdin, print its normal form and type
echo 'plus (suc zero) (suc zero)' | ./target/release/dpm-nbe nf

# print its type
echo '(A : U) -> A -> A' | ./target/release/dpm-nbe type
```

`nf` prints

```
succ (succ zero)
  :
Nat
```

Errors are printed to stderr (with the source line, a caret and the message)
and exit with status 1; parse and type errors are reported the same way:

```
<stdin>:1:17:
1 | let bad : Nat = U; U
  |                 ^
type mismatch

expected type:

  Nat

inferred type:

  U
```

## Language server and web IDE

```sh
cargo build --release -p dpm-nbe-lsp    # -> target/release/dpm-nbe-lsp
cargo test --workspace                  # library + server

node web/build.mjs                      # wasm language server -> web/www/pkg/
node web/serve.mjs                      # -> http://127.0.0.1:8140/
node web/tools/smoke.mjs                # 38 checks, no browser needed
```

The editor features all come from the language server: diagnostics are the
parser/elaborator's own errors (with the same line and column the CLI prints),
hover shows the type a binder is given by the syntax, completion only offers
names that are actually in scope, and `dpm-nbe/evaluate` returns exactly what
`dpm-nbe nf` would print.  Because the protocol layer has no dependency on
sockets or threads, the browser build runs the *same* code — the web page just
replaces `Content-Length` framing with a function call.

## The language

| Syntax | Meaning |
| --- | --- |
| `U` | universe |
| `(x : A) -> B`, `A -> B` | dependent / non-dependent function type |
| `\ p1 p2 ... . t` or `λ ...` | lambda (patterns: variables, `refl`, `(!)`) |
| `let x : A = t; u` | local definition |
| `t u` | application (juxtaposition, `plus`/`suc`/`Id` must be fully applied) |
| `Id A x y`, `refl` | identity type and its constructor |
| `Nat`, `zero`, `suc t`, `plus m n` | natural numbers |
| `-- ...`, `{- ... -}` | comments |

Notes on the concrete syntax:

* `succ` is accepted as a synonym for `suc` (the printer writes `succ`), and
  identifiers may contain `'` (the printer freshens names as `A'`), so printed
  normal forms can be fed back in.
* Patterns are only variables, `refl` and the absurd pattern `(!)`; `(!)` must
  be the last pattern, and `_` is available as a `let` binder.
* Nested block comments are not supported.

Example (a subset of the README of the original project; `tests/fixtures/`
holds the full one):

```
let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  λ A a b c refl refl . refl;

let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  λ m n refl refl . refl;

let C : (A : U) -> Id Nat (suc zero) zero -> A =
  λ A (!);   -- absurd pattern

U
```

All of these typecheck; `H` is the interesting case, see [NOTES.md](NOTES.md).

## Layout

| File | Contents |
| --- | --- |
| `src/syntax.rs` | surface/core syntax, values, neutrals, spines, substitutions |
| `src/subst.rs` | explicit substitutions (`idSub`, `insertSub`, lookup) |
| `src/eval.rs` | evaluation, forcing, primitives, conversion, quotation |
| `src/context.rs` | typing contexts and error rendering |
| `src/elab.rs` | bidirectional checking, pattern matching, unification |
| `src/parse.rs` | recursive-descent parser |
| `src/pretty.rs` | pretty printer |
| `src/main.rs` | command-line interface |
| `tests/golden.rs` | black-box suite driving the built binary |
| `lsp/src/analysis.rs` | lexer, binder scopes, hover/completion/definition/symbols, diagnostics |
| `lsp/src/protocol.rs` | JSON-RPC / LSP message handling (no I/O) |
| `lsp/src/stdio.rs` | `Content-Length` framing for the `dpm-nbe-lsp` binary |
| `web/api/dpm-nbe-wasm/` | `wasm-bindgen` binding of the server for the browser |
| `web/www/` | the web IDE (`app.js`, `lsp.js`, `editor.js`, `style.css`) |
| `vscode/` | VS Code extension: grammar, language config, desktop and web clients |

The Haskell original lives in [`../Haskell/dpm-nbe/src`](../Haskell/dpm-nbe/src)
and is the specification this port was written against; `NOTES.md` lists every
place where the port deliberately differs from it.
