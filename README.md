# dpm-nbe — Rust port

A Rust re-implementation of the Haskell project
[`KonjacSource/dpm-nbe`](../Haskell/dpm-nbe) (**dependent pattern matching
under a normalization-by-evaluation setup**), upgraded over the
original: **user-declared inductive types** (nothing is built in any more,
`Nat` is a declaration like any other), **structs** -- records as sugar over
single-constructor declarations, with `x.f` projections -- a
**predicative hierarchy of universes** `U : U1 : U2 : ...` with cumulativity,
and **holes** `_` solved by pattern unification with **pruning** (a port of
the elaboration zoo's `05-pruning`).

* ~3k lines of implementation, **no dependencies** (standard library only).
* Library + `dpm-nbe` command-line tool.
* 163 unit tests + 7 black-box tests + 25 LSP integration tests.
* [`NOTES.md`](NOTES.md) documents the algorithm and every deliberate
  difference from the Haskell original.
* [`docs/paper.md`](docs/paper.md) — a paper-form analysis of the two prune
  experiments (declaration reachability, metavariable pruning), with a
  machine-checked Lean 4 formalization of their scoping cores
  ([`docs/lean/`](docs/lean/)).

On top of the library, the repository also carries a language server and an
online IDE, so the language can be edited with diagnostics, hover, completion
and go-to-definition:

* [`lsp/`](lsp/README.md) — `dpm-nbe-lsp`, a stdio LSP server (its only
  dependency is `serde_json`).
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
echo 'data Nat : U where zero : Nat; suc : (n : Nat) -> Nat; suc zero' \
  | ./target/release/dpm-nbe nf

# print its type
echo '(A : U) -> A -> A' | ./target/release/dpm-nbe type
```

`nf` prints

```
suc zero
  :
Nat
```

Errors are printed to stderr (with the source line, a caret and the message)
and exit with status 1; parse and type errors are reported the same way:

```
<stdin>:1:48:
  |
1 | data Nat : U where zero : Nat; let bad : Nat = U; U
  |                                                ^
type mismatch

expected type:

  Nat

inferred type:

  U1
```

## Testing

```sh
cargo test --workspace                     # unit tests + the golden suite (tests/golden.rs)
cargo build --release && blackbox/run.sh   # end-to-end cases against the built CLI
blackbox/run.sh blackbox/cases/lang        # a single area: lang/, errors/ or stress/
```

The `blackbox/` tree drives the compiled binary as a black box -- program on
stdin, `nf`/`type`, stdout, stderr and exit status checked per case; the case
format and its sidecar files are documented in
[`blackbox/README.md`](blackbox/README.md).

## Language server and web IDE

```sh
cargo build --release -p dpm-nbe-lsp    # -> target/release/dpm-nbe-lsp
cargo test --workspace                  # library + server

node web/build.mjs                      # wasm language server -> web/www/pkg/
node web/serve.mjs                      # -> http://127.0.0.1:8140/
node web/tools/smoke.mjs                # 39 checks, no browser needed
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
| `U`, `U1`, `U2`, ... | the predicative tower of universes: `U i : U (i+1)` |
| `(x : A) -> B`, `A -> B` | dependent / non-dependent function type |
| `\ p1 p2 ... . t` or `λ ...` | lambda (patterns: variables, `_`, `refl`, `(!)`) |
| `let x : A = t; u` | local definition |
| `t u` | application (juxtaposition) |
| `Id A x y`, `refl` | identity type and its constructor |
| `data D params : U where c : ty; ...` | inductive type declaration |
| `struct S params : U where f : ty; ...`, `x.f` | single-constructor sugar, field projection |
| `elim D` | the eliminator generated for `D` |
| `-- ...`, `{- ... -}` | comments |

Notes on the concrete syntax:

* A `data` declaration scopes over its body like a `let`: it binds the type,
  its constructors, and nothing else.  `elim D` is a core constant whose type
  is derived from the declaration on demand.
* Identifiers may contain `'` (the printer freshens names as `A'`), and
  constructors print under the names they were declared with, so printed
  normal forms can be fed back in.
* Patterns are only variables, `refl` and the absurd pattern `(!)`; `(!)` must
  be the last pattern, and `_` is available as a wildcard binder.
* Nested block comments are not supported.

### Universes

`U` is level 0 of a predicative tower; `U1`, `U2`, ... are spelled with their
level.  The original's `U : U` is gone: `U : U1`, and a function type lives at
the maximum of its domain's and codomain's levels, so quantifying over `U`
itself lands one level up (`(A : U) -> A -> A : U1`).  Checking is *cumulative*:
where a term is checked against an expected type, `U i` fits `U j` whenever
`i <= j`, so a `(A : U1) -> A -> A` identity accepts `Nat : U` arguments.
Definitional equality (what `refl` and the pattern matcher use) stays strict.
Level numerals are bounded: the parser accepts up to `U1073741823` (every
level stays below 2^30); `U1073741824` and beyond are parse errors.

### Inductive types

Nothing is built in.  A declaration introduces a nominal type, its
constructors, and an eliminator:

```
data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;
let plus : Nat -> Nat -> Nat =
  λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;
plus (suc (suc zero)) (suc zero)
```

normalises to `suc (suc (suc zero)) : Nat`.  The eliminator is the structural
recursor `elim D : (P : D -> U) -> (method per constructor) -> (x : D) -> P x`,
with an induction hypothesis after every recursive argument; for `Nat`:

```
elim Nat : (P : Nat → U)
        → P zero
        → ((n : Nat)(n' : P n) → P (suc n))
        → (x : Nat) → P x
```

### Structs

A `struct` is sugar for a single-constructor declaration, so it lives entirely
in the surface syntax:

```
struct Pair (A : U) (B : U) : U where fst : A; snd : B;
let p : Pair Nat Nat = mkPair Nat Nat (suc zero) (suc (suc zero));
suc (p.snd)                      -- suc (suc (suc zero))
```

It elaborates to `data Pair ... where mkPair : (fst : A) -> (snd : B) ->
Pair A B` plus one projection function per field (`fst`, `snd`, each
`(A : U) (B : U) -> Pair A B -> T_i`, defined with the eliminator); `x.f`
elaborates to that eliminator applied to `x`'s type's parameters.  Field
types may not depend on earlier fields (a projection could not be typed);
use a plain `data` if you need that.  A struct must also be declared in a
universe at least as high as its field types: `struct Wrap : U1 where
carrier : U;` is accepted, but the same declaration at `U` is rejected
(`U : U1`, so the carrier's type does not fit in `U`).

Declarations may take parameters, and the eliminator abstracts them first:

```
data List (A : U) : U where nil : List A; cons : (a : A) -> (as : List A) -> List A;
let length : (A : U) -> List A -> Nat =
  λ A xs . elim List A (λ _ . Nat) zero (λ a as r . suc r) xs;
```

Declarations pick their own universe (`data Nat1 : U1 where ...`), and the
generated eliminator's motive eliminates into it.  Checking rules: the type
after the parameters must *be* a universe (no indexed families); every
constructor must return `D params`; occurrences of `D` in constructor
arguments must be strictly positive (`D params` itself, or behind function
codomains — never in a domain); two identical declarations are still distinct
types (typing is nominal).

Example (a subset of the README of the original project; `tests/fixtures/`
holds the full one):

```
data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;
let plus : Nat -> Nat -> Nat =
  λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;

let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  λ m n refl refl . refl;

let C : (A : U) -> Id Nat (suc zero) zero -> A =
  λ A (!);   -- absurd pattern

U
```

All of these typecheck; `H` is the interesting case, see [NOTES.md](NOTES.md).
With `plus` a defined function instead of a primitive, `plus m zero` is a
*stuck eliminator* on the variable `m` — which is exactly what the
delayed-substitution machinery of the original was built for.

## Layout

| File | Contents |
| --- | --- |
| `src/syntax.rs` | surface/core syntax, values, neutrals, inductive definitions |
| `src/subst.rs` | explicit substitutions (`idSub`, `insertSub`, lookup, lifting) |
| `src/eval.rs` | evaluation, forcing, eliminator reduction, conversion, quotation |
| `src/context.rs` | typing contexts and error rendering |
| `src/elab.rs` | bidirectional checking, declarations, generated eliminators, pattern matching, unification |
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
