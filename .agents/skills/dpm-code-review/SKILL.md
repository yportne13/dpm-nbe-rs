---
name: dpm-code-review
description: Use when reviewing a change to the dpm-nbe elaborator or its tooling — loads the project's standing rules and the language-level invariants a diff can silently break (stored-type closure, whnf forcing, beta-only conv, cumulativity direction, error-message contract), and the probe discipline that separates a real finding from a wrong expectation.
---

# Reviewing dpm-nbe changes

Read [AGENTS.md](../../../AGENTS.md) first; it links everything below. Judge
the diff against written rules, not memory of them. One substantiated blocker
beats a list of nits; correctness of the type theory outranks style.

## Sources of truth

- [NOTES.md](../../../NOTES.md) — numbered deviations from the Haskell
  original; the language spec deltas. A diff that changes documented behavior
  must update its deviation in the same change.
- [AGENTS.md](../../../AGENTS.md) — invariants and testing policy.
- [blackbox/findings-*.md](../../../blackbox/) — parked bugs and verified
  conventions; check whether the diff resolves or worsens a parked finding.
- `.agents/notes/` — design rationale; disagreement with a note is a design
  discussion, not a veto, but say so explicitly.

## Invariants a diff can silently break

- **Stored ctor-type closure** (`elab.rs`): after `subst_outer`, only `D` is
  free. New consumers of `CtorInfo.ty` must honor the `[D, params, fields]`
  context convention or the de Bruijn arithmetic in `elim_ty`/`infer_proj`
  breaks invisibly.
- **whnf forcing** (`eval.rs`): rigid `Ne` arms in `frc`/`frc_ne` return
  without forcing spines. Re-adding eager forcing reintroduces the O(n²)
  normalizer (see
  [the frc note](../../notes/implemented/2026-10-25-frc-strictness-port.md)).
- **Conversion strictness**: `conv` beta-only; `conv_le` cumulativity only in
  the check direction; unification and `refl` must never see `conv_le`.
  Watch for a new `check`-shaped call site accidentally using `conv`.
- **Recursive-flag consumers**: `classify_arg`'s verdict drives both
  `elim_ty` (IH binders) and `elim_step` (IH values). A change to one side
  must be mirrored by construction, not by hoping.
- **Error text and positions** are pinned by golden + blackbox tables.
- **Universe levels**: Pi = max of parts; `Id` lives where its type lives;
  ctor args bounded by the datatype's level (deviation 17); the elim motive
  lands in the datatype's own universe.

## Review method

1. Probe the behavior the diff claims to change
   ([dpm-probe](../dpm-probe/SKILL.md)); a finding without a reproducing
   program is a hypothesis, not a finding.
2. Match the diff to the narrowest suite: touched `src/` →
   `cargo test --workspace` + scoped blackbox area; touched output bytes →
   full blackbox; touched parser limits → re-pin boundary cases both sides.
3. For perf-claiming diffs, demand before/after timings on the standard
   scale cases and byte-identical `.out`s
   ([dpm-perf-check](../dpm-perf-check/SKILL.md)).
4. Check doc drift in the same pass: stale comments that misdescribe code
  (the "beta-eta" incident) are review findings too.
