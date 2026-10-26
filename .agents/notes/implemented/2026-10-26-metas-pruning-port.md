# 2026-10-26 — Metas + pruning: a no-icit port of elaboration-zoo 05-pruning

## Decision

The `pruning` branch ports [elaboration-zoo
`05-pruning`](../../../../Haskell/elaboration-zoo/05-pruning) into dpm-nbe-rs:
expression-position `_` holes, a typed metacontext, pattern unification
(invert / partial renaming / occurs check) with pruning, non-linear spine
solving and intersection (NOTES.md deviation 18).  Implicit arguments are
deliberately **not** ported -- dpm-nbe's language is explicit-only, and
icits are the *previous* zoo chapter's feature, not part of pruning proper;
a `Pruning` is therefore `Vec<bool>` instead of `Vec<Option<Icit>>`.

Key structural choices (all probe- or bench-verified, details in NOTES.md 18):

1. **`thread_local` metacontext** (`meta.rs`) instead of threading a context
   through every evaluator signature -- the zoo reads a global `IORef` from
   `force`/`eval`; reset per program in `typecheck_named`.
2. **Conversion stays pure; solving rides on top.**  `conv_or_unify{,_le}`
   runs `conv`/`conv_le` first (cumulativity intact) and only falls back to
   `unify` when metas exist.  Hole-free programs take the identical path as
   before -- the whole pre-existing suite passing unchanged is the evidence.
3. **Lazy meta-type closing** (`MetaTy::Open` + `Ctx::locals`, the zoo's
   `Locals`): eagerly closing each fresh meta's type re-quotes the context
   every time; measured 12 s at 160 nested `\ x .` (cubic-plus), 0.1 s after.
   Defines close to `Let` nodes, which keeps de Bruijn indices aligned --
   `lams`/`prune_ty` must walk *under* them (an earlier version returned the
   solution un-wrapped there, and a solved `U0` then exploded under spine
   application: `napp: impossible`).
4. **`infer` of a lambda consumes the whole var-lambda chain at once** with
   one quotation of the body type -- same oscillation, term side.
5. **Eta in `unify` only for variable-pattern lambdas**; `refl`-pattern
   lambdas re-block on a fresh variable and would loop.
6. **`check_pm`'s index unifier does not solve metas** (conservative *not
   sure*): its `Idk` outcome continues elaboration, which would observe the
   half-solved metacontext of a failed solve.

## Alternatives rejected

- Porting icits along (zoo chapter 04): doubles the elaborator surface,
  changes the language's application convention; the user asked for prune.
- Making `conv` itself side-effecting (solve during conversion): reaches
  deep inside `check_pm`'s unifier and `conv`'s many pure callers; the
  wrapper keeps every existing call site sound.
- Global `static` metacontext: the library is used from tests and LSP in
  one process; `thread_local` + reset gives per-run isolation.

## Evidence

- `cargo test --workspace`: 163 lib + 7 golden + 25 LSP, all green; the
  only golden/blackbox rows changed are the two whose pinned contract the
  feature intentionally changes (`λ x . x` now infers to an unsolved meta).
- The zoo's four example families all behave: solvable non-linear spine
  passes, unsolvable one is rejected, intersection prunes to `λ x y z . m' y`
  (reported as `unsolved ?1 : Nat → Nat`), and `(\ f . f zero) suc` /
  `(\ f . f U) g` elaborate end to end.
- Bench (best-of runs in the 2026-10-26 session): nested `\ x .` lambdas,
  10/20/40/80/160 — 12 s → 0.1 s at 160 after the lazy-closing fix.

## What would have to be true to revisit

- Implicit arguments or typed application syntax would reopen the icit
  question wholesale.
- If unsolved metas ever need to be *displayed* (LSP holes UI) rather than
  reported as errors, the end-of-program check moves to the surface that
  wants them; the metacontext already tracks positions.
