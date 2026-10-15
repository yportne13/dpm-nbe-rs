# dpm-nbe — Rust port

A Rust re-implementation of the Haskell project
[`KonjacSource/dpm-nbe`](../Haskell/dpm-nbe) (**dependent pattern matching
under a normalization-by-evaluation setup**).

* ~2.4k lines of implementation, **no dependencies** (standard library only).
* Library + `dpm-nbe` command-line tool.
* 113 unit tests + 7 black-box tests (116 checks).
* [`NOTES.md`](NOTES.md) documents the algorithm and every deliberate
  difference from the Haskell original.

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

The Haskell original lives in [`../Haskell/dpm-nbe/src`](../Haskell/dpm-nbe/src)
and is the specification this port was written against; `NOTES.md` lists every
place where the port deliberately differs from it.
