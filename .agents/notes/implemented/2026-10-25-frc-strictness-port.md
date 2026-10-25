# frc stays whnf-only on rigid neutrals

**Decision.** In `src/eval.rs`, `frc` and `frc_ne` return `Ne::Var`,
`Ne::Data`, and `Ne::Ctor` applications without forcing their spines; only
`Ne::Lam` (match) and `Ne::Elim` (elim step) force. This is a deliberate
deviation from the Haskell original and must survive future refactors.

**The bug it fixed.** The ported `frc_ne` forced every rigid spine
(`frc_sp(sp)`), and since forcing a `Ne::Ctor` re-forces *its* spine, every
`frc` of an n-successor numeral walked and re-allocated the whole chain:
O(n²) normal forms. `elim_step` forces its target once per recursion step,
`match_` forces each argument, and `quote_val` forces per node, so evaluation
and quoting each paid n forces × O(n) per force. Measured: 10 000 successors
→ 42.2 s; `fib 18` → 6.0 s; `power 2 12` → 9.6 s. After the whnf
short-circuit: 91 ms / 49 ms / 60 ms (and 65 ms for the 10 000 case once the
pretty printer also went single-buffer).

**Why the Haskell original differs.** In `Elab.hs`/`Eval.hs` the
corresponding `frcS sb <$> sp` under a rigid head is a lazy thunk that is
never demanded — laziness makes the eager spelling harmless. Porting the
spelling instead of the semantics was the defect. The same trap exists
wherever the original relies on laziness for complexity; suspect it when a
ported function walks a spine "for hygiene".

**Alternatives rejected.**

- Persistent/shared spines (Rc-list): a large restructure of `Val`/`Ne` for
  a problem the whnf fix removes; the codebase is deliberately small.
- Forcing only on first construction: same effect as whnf-on-demand but adds
  cache fields; no measurable gain over the short-circuit.

**What would have to be true to revisit.** A consumer that structurally
needs forced rigid spines at force time — at which point force lazily at
that consumer, not in `frc`. The existing structural-equality uses
(`*sp == pvs` checks in `elab.rs`) compare against freshly bound bare
variables, never delayed substitutions, so whnf is safe there today; that
assumption is worth re-checking if parameter binding ever produces non-var
values.

**Evidence.** Timings and phase splits in
[blackbox/findings-stress.md](../../../blackbox/findings-stress.md); the
output-byte oracle for purity is the full blackbox run (407 passed, outputs
unchanged) recorded with the fix.
