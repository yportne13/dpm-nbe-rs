# Porting notes

This crate started as a line-by-line port of the Haskell project in
[`../Haskell/dpm-nbe/src`](../Haskell/dpm-nbe/src), and keeps its module split,
data types, algorithm and user-visible messages wherever the two still agree.
This document records how the port works, and every place where it deliberately
differs.

The largest difference is that the port has **user-declared inductive types**
while the original hardwires `Nat`, `zero`, `suc` and a primitive `plus`: the
original's `Tm::Nat/Zero/Succ/Plus` and `Ne::Plus` are gone, replaced by the
generic `Tm::Data/Ctor/Elim` and `Ne::Data/Ctor/Elim` below.  See deviation
**14**.

## What the program does

`dpm-nbe` is a small elaborator for a type theory with `U`, `Id`, function
types and user-declared inductive types, written in the style of András
Kovács' *elaboration zoo*:

```
parse (Raw) --elaborate--> core term + semantic type --evaluate--> value --quote--> normal form
```

Values are *semantic* (`Val`), not syntax: an environment (`Env`), closures,
neutral terms (`Ne`) and **explicit substitutions** (`Sub`).  Substitution is
part of the value language (`Val::Sub`), and `frc`/`frcS` *force* a value
(undelayed substitution, run primitive reductions, apply stuck lambdas).

The point of the project is what happens when a `refl` pattern refines an
index.  Checking

```
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  λ m n refl refl . refl;  U
```

proceeds as follows:

1. the first `refl` unifies `n` with `plus m zero`: the evaluation environment
   is extended with `n := plus m zero`, where `plus` is a function defined by
   the generated eliminator, so the value is a *stuck* `elim` application;
2. the second `refl` unifies `m` with `zero`, which makes the *old* value
   `plus m zero` reducible again (`plus zero zero = zero`);
3. because the substitution is explicit and re-forced on demand, the third
   payload (`Id Nat n zero`) still checks.

A naive substitution-on-syntax implementation gets this wrong whenever a
substitution is produced *after* a value was already computed.  See the
original `README.md` and Kovács' *cctt* talk for the background.

`elab::check_pm` implements the dependent pattern matching check, and
`elab::unify`/`unify1` the first-order unification from Norell's thesis and
*Pattern Matching Without K* (this port, like the original, is a "with-K"
implementation).

## Rust-specific choices

* **Sharing instead of laziness.**  Haskell gets sharing and laziness for free;
  the port uses `Rc` in every recursive field of `Val`, `Ne` and `Closure`, so
  cloning a value is O(1).  Where laziness is load-bearing for *complexity*
  (rather than for termination), the port makes the delayed structure explicit
  instead -- see deviation 4.
* **No dependencies.**  `parse.rs` is a hand-written recursive-descent parser
  (the original uses megaparsec) and errors are rendered by hand.
* **`Result` instead of `Either`.**  The elaboration monad `M` is
  `Result<T, Error>`, where `Error` is `(String, SourcePos)`-shaped and knows how
  to render itself against the source (`Error::render`).

## Deliberate deviations from [`../Haskell/dpm-nbe/src`](../Haskell/dpm-nbe/src)

Items 1-3, 5-7 and 13 fix defects that make the original diverge, panic, lose
information or reject valid programs; item 4 restores the complexity that
laziness gave the original; items 8-12 are interface and robustness choices.
Every item is covered by tests (see the end of this file).

### 1. Forcing a lambda terminates (`eval::frc_ne`)

Haskell:

```haskell
NLam ps n cl sp -> frc $ napp (VNe $ NLam ps n cl Sp) (frc sp)
```

With an empty spine, `napp` forces its argument, which is again this lambda, so
`frc` of a *bare* lambda value diverges (a lambda wrapped in a delayed
substitution goes through the `frcS` arm instead and blocks).  Since `conv` and
`quote` force their arguments, the original cannot even compare or print a
lambda.

Port: a lambda with an empty spine is already a value and is returned as is;
with a non-empty spine the arguments are applied as before.

### 2. Pattern matching keeps the whole spine (`eval::match_`)

Haskell blocks with the *remaining* patterns next to the *original* spine and
the original arity.  Once deviation 1 is fixed this becomes observable: the
arguments consumed so far are no longer part of the patterns, so a later
application re-matches against the wrong argument.  The port blocks with the
original patterns, arity, closure and spine, so re-applying re-runs the match
from scratch and recomputes the consumed binders.

### 3. Quotation saturates partial applications (`eval::quote_ne`)

Haskell's quotation of a stuck lambda ignores its spine:

```haskell
NLam ps n cl sp -> Lam ps (quote (l + numPVars ps) (capp n cl (newVars l (numPVars ps))))
```

The original never reaches this code for a partially applied lambda (deviation
1 diverges first), but with forcing fixed it would silently drop the applied
arguments.  The port instead completes the match with a canonical argument for
every pattern that is still unmatched -- a fresh variable for a variable
pattern, `refl` for a `refl` pattern -- and prints the *remaining* patterns as
the binders of the normal form.  Examples:

| value | original (unreachable) | port |
| --- | --- | --- |
| `λ x y . plus x y` | | `λ x y . plus x y` (names preserved) |
| ... applied to `zero` | `λ x y . plus x y` | `λ y . y` |
| `H zero zero refl` | `λ m n refl refl . refl` | `λ refl . refl` |

### 4. Substitutions are composed lazily, as layers (`syntax::Sub`, `eval::subst_sub`)

Haskell composes substitutions by mapping over the entries of one of them:

```haskell
subst s2 s1@(Sub d c s) = Sub { dom = dom s1, cod = c,
                                subs = IM.union (IM.map (subst s2) s) (subs s2) }
```

`IM.map` is lazy, so each composition is a thunk that is forced (and shared) on
demand.  In a strict language the same code recurses through every nested value
eagerly: `subst_sub` was called **2^n times** while checking an n-step
transitivity proof, with running time following (trans15: 670 ms, trans18:
timeout).  The port represents a substitution as a *sequence of layers*, newest
first: composition concatenates the layer lists, and `subst::lookup_sub`
resolves a variable on demand (first layer that mentions it, then the newer
layers applied to its value).  `subst_sub` is now O(1) in the values and the
whole family is quadratic (trans60: 16 ms).

The same function also fixes a separate defect: the original stores
`dom = dom s1`, but composing `s2 : Sub Ξ Γ` with `s1 : Sub Γ Δ` must yield
`Sub Ξ Δ`, i.e. the domain of the *argument*.  The wrong field is observable
through the guard `dom s1 > cod s2`, which rejects the second iteration of
`unifyS` ("subst: (co)domain mismatch"); with the field fixed, `unify` handles
more than one equation, as its loop is written to do.

### 5. Application forces the inferred type (`elab::infer`)

Haskell:

```haskell
RApp t u -> do (t, tty) <- infer ctx t; case tty of VPi _ a b -> ...
```

`tty` is not forced, so a function whose stored type has become a delayed
substitution cannot be applied.  That situation is easy to reach: any `refl`
pattern makes `subst_ctx` wrap every type in `subst s ty`.  The port matches on
`frc(&tty)`, which is semantically transparent, and applications such as

```
let go : (A : U) (a : A) -> Nat = \ A a . zero;
let use : (A : U) (a : A) -> Id A a a -> Nat = \ A a refl . go A a; U
```

now typecheck.

### 6. Pattern matching forces the pattern's domain (`elab::check_pm`)

Haskell forces the domain before matching it against `Id`:

```haskell
PRefl -> case frc a of VId a x y -> ...
```

The port originally matched the (possibly substituted) domain directly, so a
program the original accepts was rejected.  Regression case:

```
let A : U = Id Nat zero zero;
let F : (x : Nat) -> Id Nat x x -> A -> Nat = \ x refl refl . zero; U
```

The port now forces the domain in both the `refl` and the `(!)` branch.

### 7. `Bot` has a value (`syntax::Val::Bot`)

`Tm::Bot` is the right hand side of an absurd pattern.  Haskell's `eval` has no
case for it (`error "eval: impossible"`), so as soon as an absurd lambda is
*evaluated* -- `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C` --
the original panics.  The port gives `Bot` a value, which forces, quotes,
substitutes and converts like any other atom, so such a lambda prints as
`λ A (!) . ⊥`.

### 8. Absurd-pattern hint (`elab::check_pm`)

`Elab.hs` passes the literal `"!"` to `it_should_not_be` when a `refl` pattern
meets contradictory indices, suggesting a pattern that the grammar does not
accept.  The port prints `(!)`, the actual syntax.

### 9. Error reporting and exit codes (`main.rs`)

Haskell prints *both* parse errors (megaparsec's bundle) and elaboration errors
to **stdout** and then exits 0 for parse errors (`exitSuccess`) or 0 for
elaboration errors.  The port prints every error to **stderr** and exits 1,
including malformed (non-UTF-8) input.  `--help`, no arguments, or an unknown
argument still print usage on stdout and exit 0.

The port also windows the echoed source line: lines longer than 160 characters
are shown around the caret with `…` markers, instead of echoing the whole line
(which doubles the size of the diagnostic for a pathological input).  Names
quoted in a message are truncated after 64 characters for the same reason, so
`unbound variable: <100 000 characters>` reports a bounded message rather than
the user's whole identifier.

### 10. Robustness guards

* `syntax::lvl2ix` checks for underflow instead of wrapping on `usize`, with a
  clear panic message ("variable level is out of scope").
* `syntax::lookup_env` checks bounds instead of indexing.
* `parse::MAX_DEPTH` (512) bounds **every** way the grammar can nest -- `raw`
  and `atom` recursion, the number of arguments in an application, the number
  of `(x : A)` binder groups and the number of lambda patterns -- and reports
  "expression nesting is too deep" instead of overflowing the stack.  The
  budget is shared, so a construct that goes through several of these spends
  several units per level (`suc (...)` spends three, i.e. ~170 levels; a bare
  `suc suc ...` chain reaches 512; `(((...)))` reaches ~256; chained top-level
  `let`s reach ~500).  Before this was enforced on all paths, `suc `×5000 or
  `zero `×20000 aborted the process with exit 0xC0000409.
* Because parsing, elaboration, evaluation and quotation all recurse over the
  program structure, the CLI additionally runs the whole pipeline on a thread
  with a 64 MiB stack (`lib::STACK_SIZE`, `main.rs`); library users processing
  untrusted input should do the same.  A 400-step `plus` computation costs
  ~25 ms.
* `lookup_env`/`lvl2ix` are programmer-error paths: the elaborator cannot reach
  them with well-formed input.

### 11. Parser: full backtracking, and `refl` as a whole word

The port backtracks on every alternative, so an identifier that merely starts
like a keyword is an identifier: `let letx : Nat = zero; letx` prints `zero`,
where the original commits after reading `let` and reports a parse error.

Likewise `\ reflx . zero` is a lambda with the single variable pattern `reflx`
in the port; the original's `symbol "refl"` prefix-matches and reads the
patterns `refl`, `x` (which is then rejected by the checker).

Unterminated block comments are an error in both (the original's
`skipBlockComment` fails); this was fixed in the port to match.

A leading U+FEFF (a UTF-8 byte-order mark) is treated as whitespace, because
files produced by Windows tools commonly carry one; the original rejects it.

When a keyword appears where an identifier is expected, the port reports the
position of the *start* of that token, while the original reports megaparsec's
furthest-failure position (just after the token): `suc let` is `1:5` in the port
and `1:8` in the original.  This is cosmetic -- the message is the same.

### 12. Surface-syntax extensions for reading back printed terms

The printer writes `succ` (as the original's `Pretty.hs` does) while the parser
accepts `suc`, so the original's output is not valid input.  The port accepts
`succ` as a synonym, and accepts `'` inside identifiers, because `Pretty.hs`
freshens names by appending apostrophes (`A'`).

The first of these is not purely additive: `succ` becomes a reserved word, so
`let succ : Nat = zero; succ` (which the original accepts as an ordinary
variable) no longer parses.  The alternative -- printing `suc` -- would change
every printed numeral, which is a much more visible difference, so the port
keeps the original's spelling and reserves the word.  `'` in identifiers is
purely additive.

### 13. Conversion of stuck `plus` neutrals (`eval::conv_ne`)

Haskell's `convNe` has cases for `NVar` and `NLam` only, so two syntactically
identical stuck `plus` neutrals are *not* convertible.  Conversion is therefore
not reflexive, and a well-typed program such as

```
let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \ n m . refl; f
```

is rejected with the self-contradictory "expecting plus n m equals to plus n m,
but it is not".  The port compares `Ne::Plus` structurally, which restores
reflexivity for neutrals without affecting anything that reduces.

### 14. User-declared inductive types replace the built-in `Nat`

The original hardwires `Nat`, `zero`, `suc` and a primitive `plus` (a `Plus`
neutral with a one-line recursion).  The port removes all of that: the only
built-ins left are `U`, function types, `Id`/`refl` and the absurd pattern;
everything else is declared by the program itself.

* **Syntax.**  `data D params : U where c1 : t1; ... ; body` declares a
  nominal type; the declaration elaborates to a chain of transparent `let`s
  binding the type former (`Tm::Data`), the constructors (`Tm::Ctor`) and the
  body.  `elim D` is a core constant; its type is *generated on demand* from
  the declaration (motive, one method per constructor, an induction hypothesis
  after every recursive argument, target).
* **Semantics.**  Data types, constructors and eliminators are neutral heads
  with spines (`Ne::Data/Ctor/Elim`).  Forcing an eliminator checks its
  target: a saturated constructor of the same declaration fires the method
  with the recursive results, anything else stays stuck — so delayed
  substitutions invalidate stuck eliminators exactly the way they used to
  invalidate the primitive `plus`, and the `H` example keeps working unchanged
  once `plus` is a defined function.
* **Unification.**  `unify` decomposes same-constructor spines (generalised
  injectivity of `suc`) and reports distinct constructors of one type as
  contradictory (generalised `zero ≠ suc _`), which is what makes the absurd
  pattern `λ A (!)` check against `Id Nat (suc zero) zero` for the *declared*
  `Nat`.
* **Well-formedness.**  The type after the parameters must be `U` (no indexed
  families), every constructor must return `D params`, occurrences of `D` in
  argument types must be strictly positive (as the whole argument type, or
  behind function codomains), and constructor types may not depend on
  enclosing *binders* (enclosing definitions are inlined as closed values at
  declaration time).  Declarations are nominal: two identical declarations are
  distinct types, compared by pointer equality of the definition.
* **Storage.**  A stored constructor type keeps the datatype as its single
  free variable (index = local depth), which keeps `IndDef` acyclic; the
  elaborator closes it (`close_data`, `subst_outer`) wherever a closed term is
  needed.  This convention, and the term-level `lift` that supports it, are
  new in `syntax.rs`/`subst.rs`/`elab.rs`.

Deliberately *not* supported: constructor patterns in λ (the original has no
case syntax either — elimination is `elim`), mutual or indexed inductive
types, and universes above `U`.

### 16. `struct` declarations and `x.f` projections are surface sugar

`struct S ps : U where f1 : T1; ...; fn : Tn; body` never reaches the core:
the elaborator rewrites it into the single-constructor declaration
`data S ps : U where mkS : (f1 : T1) -> .. -> (fn : Tn) -> S ps`, wrapped
around one `let` per field binding the projection
`fi : Pi ps (p : S ps) -> Ti = \u03bb ps p . elim S ps (\u03bb _ . Ti) (\u03bb f1 .. fn . fi) p`
(both written as surface syntax and re-elaborated).  A postfix `x.f` is
elaborated to the same eliminator on the spot: the parameters are read off
`x`'s type, and the field's type is computed by *evaluating* the stored
constructor domain under those parameter values -- which keeps concrete
(non-variable) parameters working where term-level index remapping would not.

Restrictions that follow: a field type may not depend on an earlier field
(the projection would be untypeable, and the declaration is rejected when
the generated projection is checked), and `x.f` requires `x`'s type to be a
single-constructor declaration.  The empty struct is the singleton type.

## Inherited limitations

* There is **no occurs check** (`Elab.hs` marks it as a TODO), so unification
  can produce a self-referential solution such as `m := plus m zero`; the
  `with-K` equation is accepted at definition time and can never be applied.
* No eta: `λ x . g x` is not convertible to a neutral `g` (no function
  extensionality), as in the original.
* The core language has no case trees: `Tm::Lam` with patterns `refl`/`(!)` is
  the only pattern form.  A stuck pattern match that cannot be completed is
  printed as the lambda itself.
* `plus`, `suc` and `Id` must be fully applied, and `refl` cannot be inferred
  (`Can't infer type for refl`).
* `(!)` cannot be the first pattern (`λ (!)` needs at least one pattern before
  it), `_` is only a `let` binder, and block comments do not nest -- all as in
  the original's parser.
* The "It's absurd pattern, you should have nothing on the right hand side."
  error is unreachable through the parser (an absurd lambda always has no right
  hand side); it is reachable through the library API and is tested directly
  (`elab::tests::absurd_pattern_rejects_a_right_hand_side`).
* `⊥` (the printed body of an absurd pattern) is **display only**: there is no
  syntax for it, so the normal form of an absurd-pattern lambda cannot be fed
  back in.  The original cannot print such a value at all.
* `parse::MAX_DEPTH` is deliberately conservative (512 units, shared across
  nesting forms), because the parser itself runs on the caller's stack: about
  256 nested parentheses, ~170 `suc (...)` levels, or 512 flat arguments /
  patterns / binders.  The CLI's 64 MiB thread is defence in depth, not the
  limit.
* A program can still *build* an exponentially large normal form, and then
  evaluation, quotation and printing exhaust time and stack by construction:
  `let d : Nat -> Nat = \ n . plus n n; d (d (... (suc zero)))` doubles its
  result at every application (k = 16 takes ~3 s, k = 17 exhausts the stack).
  This is inherent to the program, not to the depth guard.

## Tests

`cargo test` runs 142 unit tests inside `src/` plus the `tests/golden.rs`
black-box suite (7 tests, ~120 table checks: success cases including the
arithmetic prelude, `Bool`/`List` elimination, the README programs and
generated transitivity proofs, declaration errors, elaboration errors with
byte-pinned positions and carets, parse errors, the usage contract, BOM
acceptance and the long-line window).

Unit tests cover: de Bruijn levels/indices and environments, substitution
lookup/composition/forcing/lifting, eliminator reduction (firing, stuck,
partial application, over-application), application and pattern matching
(including partial application, saturated matching and stuck matches),
conversion (including stuck-eliminator reflexivity and nominal distinctness of
identical declarations), quotation, unification (all `UnifyRes` cases,
constructor injectivity and conflict, multi-equation threading), the generated
eliminator types of `Nat`/`List`/the empty type, parser structure, error
positions, every nesting limit and comment handling, error rendering, pretty
printing (precedence, freshness), and the whole elaboration pipeline: the
README examples, declarations (scoping, shadowing, nominal typing, positivity,
indexed families rejected, enclosing binders rejected), `Bool`/`List`
elimination, `genTransN` for n = 1..12 plus a scaling test up to n = 30, every
error message, the definitional-equality cases, the two output modes, and the
fixed regressions described above.  The LSP suite adds 25 integration tests,
the web IDE 39 + 29 smoke checks.
