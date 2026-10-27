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
laziness gave the original; items 8-12 are interface and robustness choices;
item 17 keeps constructor declarations inside their universe; item 18 ports
metavariables and pruning from the elaboration zoo; item 19 adds checked
`let rec` fixpoints with a structural termination check; item 20 adds
typeclasses resolved by that very unifier.
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
  budget is shared, and forms spend it at different rates: parenthesised
  nesting (applications, `(...)`, `suc (suc (... zero))` literals) costs ~2
  units per level, binder bodies (`λ`, `let`) ~1, while projections, data
  parameter groups and flat argument width have their own `> 512` counters.
  Measured last-pass/first-fail on the release binary: `(((...)))` 255/256,
  `λ`s nested in one `let` 509/510, chained `let`s 510/511, projection chains
  512/513 and flat argument width 511/512; a program prelude shifts the
  nesting forms down by the units it has already spent (253/254, 506/507,
  507/508 there).  The `blackbox/` suite pins both sides of every boundary;
  `blackbox/findings-stress.md` is the executable source of truth.  Before
  this was enforced on all paths, `suc `×5000 or `zero `×20000 aborted the
  process with exit 0xC0000409.
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

### 12. Printed terms read back: `suc` everywhere, `'` in identifiers

The original's `Pretty.hs` writes `succ` while its parser accepts only `suc`,
so the original's printed output is not valid input.  The port prints what it
parses: numerals are written `suc zero`, `suc (suc zero)`.  `succ` is not a
reserved word -- with `Nat` declared, `succ zero` fails with `unbound
variable: succ`, and `let succ : Nat = zero; succ` elaborates like any other
name.  The port also accepts `'` inside identifiers, because `Pretty.hs`
freshens names by appending apostrophes (`A'`).

The cost is a visible spelling difference against the original (`suc zero`
where `Pretty.hs` writes `succ zero`); the gain is that the port's output
round-trips as input without reserving any word.  `'` in identifiers is
purely additive.

### 13. Conversion of stuck neutrals (`eval::conv_ne`)

Haskell's `convNe` has cases for `NVar` and `NLam` only, so two syntactically
identical stuck `plus` neutrals are *not* convertible.  Conversion is therefore
not reflexive, and a well-typed program such as

```
let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \ n m . refl; f
```

is rejected with the self-contradictory "expecting plus n m equals to plus n m,
but it is not".  The port compares stuck neutrals structurally -- `Ne::Plus`
originally, and since deviation 14 the `Ne::Data`/`Ne::Ctor`/`Ne::Elim` heads,
so the same program with a *declared* `Nat` and a defined `plus` is again
accepted -- which restores reflexivity for neutrals without affecting anything
that reduces.  Conversion is nonetheless **beta-only**: there is no eta, so a
lambda is never convertible to a neutral of function type -- `Id (Nat -> Nat)
(\ x . f x) f` is rejected with "expecting λ x . f x equals to f, but it is
not", and `Id (Nat -> Nat) (\ x . suc x) suc` likewise -- as in the original.

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

### 15. A predicative tower of universes with cumulativity

The original has a single self-filled universe (`U : U`), which makes its
theory inconsistent.  The port replaces it with a tower: `Tm::U(l)` /
`Val::U(l)`, written `U` for level 0 and `U1`, `U2`, ... above it, with
`U l : U (l+1)`.  Level numerals are bounded: `parse::MAX_LEVEL = 2^30 - 1`
is the largest the parser accepts (every level stays below 2^30), and
`U1073741824` up is a parse error.

* **Where levels come from.**  A function type lives at the maximum of its
  domain's and codomain's levels; `Id A x y` lives where `A` lives; a `data`
  declaration records the universe it was declared in, types the former as
  `D : U level` and lets the generated eliminator's motive eliminate into
  `U level`.  Elaborating a type position now *infers* the term and reads the
  level off its type (which is always a universe — that is what makes it a
  type), instead of checking against a fixed `U`.
* **Cumulativity is check-direction only.**  Where a term is checked against
  an expected type, `U i` fits `U j` whenever `i <= j` and a function type may
  return a smaller type than expected (`eval::conv_le`); definitional
  equality (`refl`, unification) keeps using the strict `conv`, so `U` and
  `U1` are never *the same* type.  Consequence: the port rejects
  `let A : U = U` (the original accepted it), and the program-final `U` prints
  as `U : U1`.
* Universe polymorphism (a `Level` variable), an impredicative `Prop`, and
  `Setomega`-style fixes are all deliberately out of scope.

### 16. `struct` declarations and `x.f` projections are surface sugar

`struct S ps : U where f1 : T1; ...; fn : Tn; body` never reaches the core:
the elaborator rewrites it into the single-constructor declaration
`data S ps : U where mkS : (f1 : T1) -> .. -> (fn : Tn) -> S ps`, wrapped
around one `let` per field binding the projection
`fi : Pi ps (p : S ps) -> Ti = λ ps p . elim S ps (λ _ . Ti) (λ f1 .. fn . fi) p`
(both written as surface syntax and re-elaborated).  A postfix `x.f` is
elaborated to the same eliminator on the spot: the parameters are read off
`x`'s type, and the field's type is computed by *evaluating* the stored
constructor domain under those parameter values -- which keeps concrete
(non-variable) parameters working where term-level index remapping would not.

Restrictions that follow: a field type may not depend on an earlier field
(the projection would be untypeable, and the declaration is rejected when the
generated projection is checked), the declaration must sit in a
universe at least as high as its field types (`struct Wrap : U1 where
carrier : U;` is accepted; at `U` the field type `U : U1` does not fit, and
the declaration is rejected), and `x.f` requires `x`'s type to be a
single-constructor declaration.  The empty struct is the singleton type.

### 17. A datatype sits at least as high as what its constructors store (`elab::elab_data`)

A constructor's type is a telescope of function types ending in `D ps`.  A
function type lives at the maximum of its domain's and codomain's levels, and
the result `D ps` sits at `D`'s own level, so the level of the constructor
type is bounded below by every argument domain: whatever a constructor stores
forces the declaration's level at least that high.  The port enforces this
with a single comparison (`clvl > level`) while checking the declaration:

```
data Wrap : U where mk : U -> Wrap; Wrap
```

is rejected at the declaration with `constructor mk of Wrap may not store
arguments from a higher universe (declare Wrap in a higher universe)` -- the
stored `U` lives in `U1`.  The legal shape moves the declaration up:

```
data R : U1 where base : R; mk : (U -> R) -> R; R
```

Here the argument type `(U -> R)` lives in `U1`, exactly where `R` sits, and
the declaration elaborates (`R : U1`).  The comparison is on levels, so a
*smaller* argument is fine (`data W : U1 where mk : Nat -> W;` checks, since
`Nat : U`), every rung of the tower is guarded (`mk : U1 -> W` needs `W : U2`)
and parameters are covered (`data Box (A : U1) : U where mk : A -> Box A` is
rejected the same way).  `struct` declarations meet the same guard through
their generated constructor: `struct Wrap : U where carrier : U;` reports
`constructor mkWrap of Wrap ...`, while `struct Wrap : U1 where carrier : U;`
is accepted -- the "universe at least as high as its field types" restriction
of deviation 16 is this guard.

`R` also shows how a `D`-occurrence behind a function *codomain* is treated:
strictly positive, but not a recursive argument, so the generated
eliminator's method for `mk` takes exactly the constructor's one argument and
no induction hypothesis:

```
elim R : (P : R → U1) → P base → ((U → R) → P (mk _)) → (x : R) → P x
```

### 18. Metavariables and pruning, from the elaboration zoo (`meta`, `unify`)

The port grows holes: `_` in expression position elaborates to a fresh
*metavariable*, solved by pattern unification with **pruning** -- a port of
[`05-pruning`](../../Haskell/elaboration-zoo/05-pruning) from the elaboration
zoo, minus implicit arguments (dpm-nbe is explicit-only, so a pruning is a
plain `Vec<bool>`).  A `_` in binder or pattern position stays a wildcard;
only the expression-position `_` was free to become a hole (it used to be a
parse error).

What works, in decreasing order of visibility:

* `let x : _ = zero; suc x` checks: the annotation hole is solved to `Nat`
  from the checked value, and `refl` solves holes inside identity types
  (`let p : Id Nat _ zero = refl` gives `Id Nat zero zero`).
* Variable-only lambdas are now *inferable* (`λ f . f U`, the zoo's pruning
  example): the domain is a fresh meta, and applying `f` refines it to a
  fresh function type `?0 =? (x : ?1 f) → ?2 f x`, where the renaming prunes
  `f` off `?1`/`?2` by solving them as projections of fresh metas.  A lambda
  with `refl`/`(!)` patterns still needs a checked position.
* Non-linear spines solve when the duplicated arguments can be pruned off
  the meta's *type* (`m A A =? λ x y . y` above m of a type not mentioning
  `A`/`B` solves `m := λ A B x y . y`), and are rejected when they cannot.
* `?m sp =? ?m sp'` with both spines renamings prunes the arguments the two
  disagree on (intersection); `?m sp =? ?m' sp'` inverts the longer spine.

Unification failure is a hard error and may leave earlier solutions in the
metacontext (as in the zoo); no later check runs, so nothing observes the
partial state.  A program whose holes stay unsolved is an error reported at
the hole's position: `unsolved metavariable ?{m}`.

Porting decisions worth recording:

* The metacontext lives in a `thread_local` (`meta`), reset by
  `typecheck_named` -- the zoo reads a global `IORef` from `force` and
  `eval`, and threading a context through every evaluator signature would
  churn the whole port for the same effect.
* `Ne::Meta` is a *flexible* head: unsolved it is its own whnf, solved it
  unfolds by applying the solution to the spine.  This does not violate the
  whnf-only-on-rigid convention of the frc port (`eval::frc`); metas are not
  rigid, and unfolding them is required for solved holes to be transparent.
* Conversion stays pure (`conv`/`conv_le` compare unsolved metas only
  structurally, and cannot solve); `unify::conv_or_unify{,_le}` is the
  check-position comparison: pure conversion first -- cumulativity intact --
  and only a failure with metas in scope falls back to unification, which
  can solve a `Pi` codomain deep inside.  Without metas the fallback is
  skipped entirely, so hole-free programs pay nothing.
* Meta types are closed lazily.  `Ctx` keeps the quoted context chain (the
  zoo's `Locals`; defines become `Let`s in the closed type, which is what
  keeps de Bruijn indices aligned), and a fresh meta stores its type open;
  only pruning, `lams` and the unsolved report close it.  Closing eagerly
  instead makes every binder over a hole re-quote the context, which
  measured cubic on deeply nested lambdas (160 nested `\ x .` took 12 s
  before, 0.1 s after).
* `infer` of a lambda consumes the whole chain of variable lambdas at once
  and closes the type with a single quotation -- the term-level counterpart
  of the same oscillation.
* Eta in unification expands only variable-pattern lambdas: a `refl`-pattern
  lambda applied to a fresh variable would re-block forever.  Pattern
  lambdas fall back to structural conversion.
* `check_pm`'s index unification (the `refl` machinery) treats metas
  conservatively: an equation it cannot decide reports *not sure* rather
  than solving, because its `UnifyRes::Idk` outcome would otherwise observe
  the half-solved metacontext of a failed solve.

Holes standing for types in *inference* position are solved to `U`-level-0
types only (`infer_type` unifies the type-of-type meta with `U`):
`let x : _ = zero` works for any small type, a hole that should mean `U`
itself needs an annotation.  Tests: the `tests::holes_*`/`*prun*` families in
`src/lib.rs`, `unify`'s own tests, the golden `nf_hole_*`/`meta` rows, and
`blackbox/cases/lang/meta-*.dpm`.  The scoping core of the algorithm —
the partial-renaming inversion theorem and the index-discipline invariant —
is machine-checked in Lean 4; see
[docs/paper.md](docs/paper.md) §4–5 and [docs/lean/](docs/lean/).

### 19. `let rec`: checked fixpoints with a structural termination check (`fix`, `unify`)

The port grows *general* recursive definitions: `let rec f : T = body`
admits `f` being called recursively in `body` — beyond what an eliminator's
induction hypotheses express — after a termination check passes.  On
admission the definition becomes a **fixpoint** (`Tm::FixRef`, `Ne::Fix`):
a closed λ-telescope stored in a fixcontext (`fix`, the same shape of
per-program table the metacontext has), whose self-calls go through the
table lookup itself.  The fixpoint *reduces* only when its designated
recursive argument — the first spine entry — forces to a constructor
(`eval::fix_step`), so computation stays stuck on neutral arguments exactly
like an eliminator does.

The check is structural, and stated over the *closed elaborated body*
rather than the surface syntax:

* The candidates are the argument positions of `f`'s type whose domain is a
  declared datatype; the check tries each and accepts the first where every
  self-call decreases (`elab::calls_decrease`).
* The designated argument's binder gets depth 0; an eliminator method's
  constructor-argument binders get their target's depth plus one (the
  method's induction hypotheses stay depth-less).  A self-call `f ... a ...`
  must be a *saturated* application whose argument at the candidate
  position is a variable of depth ≥ 1 — a strict subterm.  Calls on the
  argument itself, on constructed terms, on induction hypotheses, or
  under-applied/bare self-references are rejected.
* Well-foundedness: unfolding happens only on constructors, and each unfold
  replaces the recursive argument by a strict subterm of it — the depths the
  checker tracked — so the subterm ordering terminates every chain of
  unfolds.  This is the classical structural-recursion soundness argument,
  at the scope of one fixpoint.

Restrictions that follow from the design:

* The stored body must be **closed** (like stored constructor types):
  `let rec` under binders is fine only when neither the type nor the body
  actually mentions them; otherwise `may not depend on an enclosing binder`.
* Mutual recursion is impossible by construction: the declaration chain is
  linear, so `f` can only call *earlier* definitions.
* A hole inside a fixpoint body is conservative: a self-call hiding behind
  a metavariable is not a saturated application, so it is rejected.

Two definitions of the same shape are distinct fixpoints (like distinct
datatype declarations); a stuck `f x` prints under `f`'s name.  Tests: the
`fixpoints_*`/`structural_recursion_*` families in `src/lib.rs`, `fix`'s
own tests, the golden `nf_rec_*`/`err_rec_*` rows, and
`blackbox/cases/lang|errors/rec-*.dpm`.

### 20. Typeclasses: struct sugar, an instance table, and resolution by unification (`tc`)

The port grows typeclasses, again as sugar over what is already there.  A
`class C ps : ty where m1 : T1; ...` declaration **is** a struct (deviation
16): the single-constructor datatype `C`, the constructor `mkC`, and one
projection per method.  What it adds is a registry entry, and that one entry
changes how the projections are *used*: a method reference like `show` has
type `(A : U)(q : Show A) -> ...`, and its class-typed domain is filled
**automatically** at the application site -- `show Nat zero` elaborates to
`show Nat <dictionary> zero`, the dictionary coming from instance
resolution.  The explicit paths are the field projection `d.show` on a
dictionary value (which never goes through an application) and the `[T]`
expression, which resolves by hand and prints the found dictionary.

An `instance i : T = v` declaration is a `let` whose type must end in a
class application; the checked value and type are closed (the same
no-enclosing-binders rule as stored constructor types and `let rec` bodies)
and pushed to a thread-local table, scanned **newest first** -- a later
declaration shadows an earlier one for the same target.

Resolution is a depth-limited backtracking search, and it is the ported
pattern unification doing the work: for each table entry, fresh
metavariables are created for the instance type's whole telescope, the
codomain is unified with the target (metas on the instance side get solved
-- this is `unify::unify` from deviation 18), the ordinary domains must come
out solved and become the instance's arguments, and each *class* domain is
resolved recursively, its solution written back into that meta.  A failed
entry rolls the metacontext back -- `meta::snapshot`/`restore` exist for
exactly this -- so half-solved attempts are never observed.  The search
depth is bounded (`MAX_DEPTH = 64`), which both terminates the recursion
and rejects cyclic instance sets (`instance : Eq A -> Eq A` searches to the
bound and fails).

Restrictions that follow:

* A class's methods are its struct fields, so the deviation-16 field rules
  apply: no duplicate methods, a method type may not depend on an earlier
  method.
* Instances must close (top level, or under binders they do not mention);
  a local instance that closes is fine and takes precedence while it is in
  scope, being the newest entry.
* Coherence is **not** enforced: two matching instances are resolved by
  table order, not by uniqueness, so overlapping instances can produce
  different dictionaries at different scopes.
* No superclasses, no prioritised overlap resolution, no `deriving`.

The class registers **before** its body elaborates (`elab_data`'s `class`
flag): the body is the rest of the program, and its instances must already
see the class -- an earlier version registered after the body and every
instance of the class itself failed resolution.  Tests: the `instances_*`/
`resolution_*` families in `src/lib.rs`, `tc`'s own tests, the golden
`nf_tc_*`/`err_tc_*` rows, and `blackbox/cases/lang|errors/tc-*.dpm`.

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
  nesting forms), because the parser itself runs on the caller's stack: 255
  nested parentheses, 254 `suc (...)` levels under a minimal `Nat` prelude,
  ~509 nested binder bodies or 511 flat arguments (deviation 10 records the
  measured boundaries).  The CLI's 64 MiB thread is defence in depth, not the
  limit.
* A program can still *build* an exponentially large normal form, and then
  evaluation, quotation and printing exhaust time and stack by construction:
  `let d : Nat -> Nat = \ n . plus n n; d (d (... (suc zero)))` doubles its
  result at every application (k = 16 takes ~3 s, k = 17 exhausts the stack).
  This is inherent to the program, not to the depth guard.

## Tests

`cargo test` runs 177 unit tests inside `src/` plus the `tests/golden.rs`
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
error message, the definitional-equality cases, the output modes, and the
fixed regressions described above.  The metavariable tests cover the zoo's
example set (holes in annotations and identity types, lambda inference with
pruning, solvable and unsolvable non-linear spines, intersection pruning)
plus the unsolved-meta report and its position.  The `let rec` tests cover the
checker (acceptance through deep subterms and non-first argument
positions, rejection of non-strict, constructed and higher-order calls,
the closedness restriction), reduction on constructor arguments, and
conversion of stuck fixpoints.  The LSP suite adds 25
integration tests, the web IDE 39 + 29 smoke checks.
