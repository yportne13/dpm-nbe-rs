# 2026-10-28 — `let rec` fixpoints: reduction gated on constructors, termination checked structurally

## Decision

The `letrec` branch adds general recursive definitions (NOTES.md deviation
19).  A `let rec f : T = body` becomes a **fixpoint** — a closed
λ-telescope in a per-program fixcontext (`fix`, the same shape as the
metacontext) — and reduction is **gated**: the fixpoint unfolds only when
its designated recursive argument forces to a constructor (`fix_step`),
exactly the guard eliminators have on their targets.  Termination is the
checker's theorem, not the reducer's.

Three choices carry the design:

1. **The check runs on the closed elaborated body**, not the surface
   syntax.  Depth tracking is purely structural: the designated argument's
   binder is depth 0, and each eliminator method's constructor-argument
   binders get their target's depth + 1 (induction hypotheses stay
   depth-less).  A self-call must be saturated with a depth-≥1 variable at
   the candidate position.  No type information is consulted during the
   walk — the eliminator structure *is* the subterm evidence, which keeps
   the checker independent of the metacontext and of type-whnf subtleties.
2. **All inductive argument positions are candidates**, tried in order;
   acceptance is the first position where every call decreases.  This is
   what makes non-first-argument recursion (the `swap a m` shape) work
   without nested fixpoints, and it costs one linear walk per candidate.
3. **The body closes like a stored constructor type**: checked with `f` as
   a fresh variable, quoted (inlining defines), with that variable's
   occurrences rewritten to `FixRef(id)` — so the self-reference *is* the
   table lookup, unfolding needs no environment, and two occurrences of
   the same fixpoint compare by id.  Any other free variable that survives
   quoting is an enclosing binder the fixpoint may not capture (the same
   restriction constructor types have).

## Alternatives rejected

- **Desugaring `let rec` into eliminator applications**: only covers calls
  that are exactly the method's recursive pattern variable — a subset of
  what elim already expresses, so the feature would add nothing.  The
  fixpoint genuinely extends expressiveness (`half` descending two
  levels, non-first-argument positions).
- **Guarded fix in the core with no admission check** (Lean-style `fix`
  exposed to users): every unsound program becomes the user's fault and
  normalization is lost; the language's promise so far is that programs
  normalize.
- **Size-change or lexicographic checking**: strictly more acceptance, at
  the cost of a graph algorithm and a much harder soundness story; the
  classical structural check is what the repo can actually verify
  (the subterm-depth argument is a page, and stateable against the Lean
  formalization's spine machinery if ever needed).

## Evidence

- Checker semantics probe-verified: `double`/`half` (deep subterm) and
  `swap a m` (second argument position) accept and compute; calls on the
  argument itself, on constructed terms, on induction hypotheses,
  under-applied self-references, and recursion with no inductive argument
  all reject with the documented messages.
- Reduction semantics: unfolding only on constructor arguments; a stuck
  `f n` under a binder prints as `f n` and is convertible with itself
  (`refl` on `Id Nat (double n) (double n)` checks).
- 170 workspace unit tests (5 new `fixpoints_*`/`structural_recursion_*`
  + `fix` roundtrip), 3 golden rows, 6 blackbox cases — all green; full
  blackbox 419/0/3 at commit time.
- Implementation post mortem worth keeping: the first `dec_method`
  double-counted induction hypotheses (they are already *in* the method's
  pattern list) and then mis-ordered the depth environment (`insert(0)`
  already reverses; an explicit `.rev()` swapped m and r).  Both produced
  wrong-vertex rejections that the probe matrix caught immediately — the
  matrix (accept/deep/position-shift/reject-family) is the regression
  net for any future checker change.

## What would have to be true to revisit

- Mutual recursion would need either a simultaneous fixpoint or a
  strongly-connected-component pass over the declaration chain; the linear
  chain currently makes it impossible by construction.
- Size-change termination would subsume the structural check; revisit if
  real programs hit its expressiveness wall (e.g. Ackermann needs two
  arguments and lexicographic order — the current check rejects it).
