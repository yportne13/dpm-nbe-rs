# Findings — semantic correctness (`cases/lang/`)

Written against the pinned release binary
`target/release/dpm-nbe.exe` (sha256 `7a919ad63518174b`).
66 new cases (`arith-*` 17, `elim-*` 16, `struct-*` 11, `univ-*` 10,
`id-*` 9, `conv-*` 3) plus the two seeds; all pass, nothing skipped.

## Bugs

**None.**  No case exited 101, printed `internal panic`, hung, or produced
a semantically wrong answer.  Every numeral was checked against
`gen.mjs` (an independent oracle that renders Peano numerals in the
printer's exact shape), including the big ones: `2^10 = 1024`,
`3^5 = 243`, `fib 15 = 610`, and the 3-iterated tripling `3^3 = 27`.

## Verified conventions (surprises that turned out to be correct)

These initially surprised me; each was re-run and confirmed, and none is
semantically wrong.  They are pinned by cases so regressions will show.

* **Numeral printing drops the innermost parens.**  The numeral 1 prints
  as `suc zero`, not `suc (zero)`; only *nested* applications get
  parentheses (`suc (suc zero)`).  My first oracle assumed full parens
  and "failed" against the binary — the binary was right.  (`gen.mjs`
  encodes the correct shape.)
* **A zero-constructor declaration is written `where <body>`.**  The
  `where` keyword is followed directly by the body when the constructor
  list is empty: `data Empty : U where elim Empty` (pinned by
  `elim-empty-head`).  `where;` does not parse.
* **Anonymous constructor binders print as `_`.**  With
  `suc : Nat -> Nat`, the elim head prints its method type as
  `Nat → P _ → P (suc _)`; with `suc : (n : Nat) -> Nat` it prints
  `(n : Nat)(n' : P n) → P (suc n)` (as `tests/golden.rs` pins).  The
  underscore form is display-ambiguous to a human (two binders both
  named `_`) but the elaborated type is the same; all elimination
  programs through the anonymous form typecheck correctly.  Pinned by
  `elim-nat-head`.
* **Quotation freshens colliding binders with `'`.**  In
  `arith-mult-1-stuck` the method binder `\ m r` prints as
  `λ n' r` under the outer `λ n` — consistent with NOTES.md deviation 12
  (the parser accepts `'` in identifiers so output is re-feedable).
* **A struct let-value that is the bare eliminator prints un-lambda'd.**
  `let idb : Bool -> Bool = elim Bool (\ _ . Bool) true false; idb`
  prints as the stuck neutral `elim Bool (λ _ . Bool) true false`
  (`elim-bool-id-value`), and a motive-first partial elim prints
  `elim Nat (λ _ . Nat)` (`elim-partial-value`).
* **A stuck surface projection prints as its underlying eliminator**,
  with the field names as method binders:
  `λ p . elim Pair (λ _ . Nat) (λ fst snd . snd) p`
  (`struct-stuck-proj`, `struct-elim-head`).

## Coverage gaps left deliberately

* **Eta-equivalence: not tested.**  NOTES.md documents "No eta:
  `λ x . g x` is not convertible to a neutral `g`" as an inherited
  limitation, so there is no *positive* semantic case to pin, and the
  rejection is an error-path behaviour owned by the `errors/` suite.
* **Error/diagnostic behaviour and exit codes** are owned by
  `cases/errors/`; the only exit-1 runs during development were my own
  malformed probes (missing declaration, wrong method arity — the
  messages were correct).

## Helper

`gen.mjs` — numeral oracle / `.out` generator
(`node blackbox/cases/lang/gen.mjs numeral 1024`,
`... out 610 Nat FILE`, `... check FILE 610`).

## Extension: eta, conversion corners, algebra laws

19 new runnable cases (`eta-*` 5, `conv2-*` 6, `laws-*` 8) plus one
parked `eta-var-head.dpm.skip`; runner: 87 pass, 0 fail, 1 skip,
against the same pinned binary (sha256 `7a919ad63518174b`).

### Finding — eval.rs calls `conv` "beta-eta", but eta never holds

`src/eval.rs:379` heads the section with

    // Conversion checking (beta-eta)

and `src/eval.rs:399` documents the function as

    /// Beta-eta conversion.  Precondition: both values have the same type.

Yet in every shape where one side is a lambda and the other the neutral
it eta-expands to, conversion is rejected with exit 1 and
`expecting <lhs> equals to <rhs>, but it is not`.  Repros (all
verified against the pinned binary):

1. variable head, the canonical shape (parked as `eta-var-head.dpm.skip`):
   `let e : (f : Nat -> Nat) -> Id (Nat -> Nat) (λ x . f x) f = λ f . refl`
   → `expecting λ x . f x equals to f, but it is not`.
2. constructor head: `Id (Nat -> Nat) (λ x . suc x) suc = refl`
   → `expecting λ x . suc x equals to suc, but it is not`.
3. partially applied eliminator head:
   `Id (Nat -> Nat) (λ n . elim Nat (λ _ . Nat) zero (λ m r . m) n)
   (elim Nat (λ _ . Nat) zero (λ m r . m)) = refl` → rejected the same
   way (a lambda vs the stuck `Ne::Elim` neutral).
4. shorter lambda vs multi-pattern lambda:
   `Id (Nat -> Nat -> Nat) (λ x . plus x) plus = refl` (well-typed!)
   → rejected, and both sides *print* as lambdas (`λ x . λ b . elim … x`
   vs `λ a b . elim … a`) because quotation eta-expands partial
   applications (NOTES deviation 3) — the display agrees, the
   conversion checker does not.

NOTES.md:439 documents exactly this ("No eta: `λ x . g x` is not
convertible to a neutral `g`"), so the behaviour is deliberate and
NOTES is correct; the two eval.rs comments are the defect.  The parked
case's sidecars pin today's rejection (`.code` 1, stderr fragment
`equals to f, but it is not`); if eta is ever implemented, unskip it
with pass sidecars instead.

**Boundary, mapped by the passing `eta-*` cases:** eta holds *iff both
sides are lambdas* — `conv` opens both closures under fresh variables
and compares the bodies, so alpha-renamed expansions agree at every
head (`eta-alpha-var-head`, `eta-alpha-ctor-head`), the full expansion
of a defined function equals it (`eta-full-expansion-plus`), and the
surface projection lambda equals the struct-bound projection function
(`eta-projection-fn`, both literally lambdas).  It fails as soon as
one side is a neutral: variable, constructor, partially applied
`elim`, or a partially applied shorter lambda.  `conv_le` cannot even
pose the question — it only compares types (`U` vs `U`, `Pi` codomains
contravariantly-strict in the domain) and falls through to strict
`conv` for every term, so no check-direction subtyping creates eta
anywhere.  Printing is a different story: `eta-quote-total-plus` pins
that the *normal form* of bare `plus` is the saturated
`λ a b . elim Nat (λ _ . Nat) b (λ n r . suc r) a`.

### Verified conventions added

* **Stuck elims with merely-convertible motives agree**: a
  transparent `let m : Nat -> U = λ _ . Nat` motive and the inline
  `(λ _ . Nat)` motive give convertible stuck neutrals, so refl proves
  the two elims equal under a binder (`conv2-stuck-elim-motive-conv`).
  (The other spelling — a *neutral* motive variable vs the lambda — is
  not convertible, which is the same no-eta boundary as above.)
* **`p . fst` IS the eliminator it desugars to**: refl proves
  `Id Nat (p . fst) (elim Pair (λ _ . Nat) (λ fst snd . fst) p)` under
  a binder (`conv2-proj-vs-elim`), pinning NOTES deviation 16's
  "the same term" claim at the conversion level.
* **Cumulativity at Id endpoints**: `Id U1 Nat Nat = refl` typechecks
  (endpoints live a level below the Id's type argument, check
  direction uses `conv_le`; `conv2-id-u1-nat-nat`), while `Id U U U`
  is rejected with `expected type: U` / `inferred type: U1`, exit 1
  (`conv2-id-u-u-reject`).  The legal "U equals U" stays
  `Id U1 U U` (already pinned by `id-refl-at-u1`).
* **Let chains are fully transparent**: six shadowed definitions of
  one name (`conv2-let-shadow-chain`) and a six-link chain of distinct
  names (`conv2-let-chain-distinct`) both unfold all the way for refl.
* **Arithmetic laws** (`laws-*`): closed instances of plus-comm
  (3+5 = 5+3), plus-assoc, mult-comm (2·3 = 3·2), mult-dist
  (2·(3+4) = 2·3+2·4) and minus (3−2 = 1) all prove by `refl` —
  conversion does the proving after both sides normalise to numerals.
  `plus zero n = n` holds *definitionally* for a neutral `n` because
  the eliminator recurses on its first argument and fires on the
  `zero` constructor; the printed Id type reduces to `Id Nat n n`
  (`laws-plus-zero-left`), and `pred (suc n) = n` behaves the same
  (`laws-pred-suc-roundtrip`).  The other side, `plus n zero`, stays
  the stuck eliminator `λ n . elim Nat (λ _ . Nat) zero (λ n' r . suc r) n`
  (`laws-plus-n-zero-stuck`; same stuck shape `arith-mult-1-stuck`
  reaches via `mult 1 n`) — that is the no-induction boundary, pinned
  as a normal form rather than a failed `refl`.

The coverage gap recorded above ("Eta-equivalence: not tested") is
closed by this extension: the rejection is pinned as the parked
`eta-var-head.dpm.skip` case plus this finding, and the
lambda-vs-lambda side of the boundary is pinned by four passing cases.
