# 2026-10-25 — `prune` is parse-only reachability, not elaboration filtering

## Decision

`dpm-nbe prune` (deviation 18, [NOTES.md](../../../NOTES.md)) computes which
top-level declarations the goal reaches by **free-name resolution over the
parsed `Raw` tree**, prints the kept chain back as surface syntax, and never
typechecks. Reachability is lexical and innermost-first, mirroring
`Ctx::lookup`, which is the whole soundness argument: the innermost binder
resolves a use, it is kept whenever the use is, so dropped declarations
cannot rebind anything and the pruned program elaborates identically.

## Alternatives rejected

- **Elaborate first, prune the core term.** Rejected: it would reject the
  broken-but-unreachable declarations pruning exists to tolerate, and the
  core term no longer knows which surface declaration provided which name.
- **Skip unreachable declarations during `nf`/`type`.** Rejected: it changes
  the error contract of existing modes (errors in unused code would vanish);
  golden and blackbox pin that behavior. `prune` is additive instead.
- **Source-slice output** (cut the kept declarations' original bytes by
  position) instead of reprinting the AST. Rejected: comments and layout
  would survive, but reconstruction must then re-derive separators and the
  goal's span; reprinting is one deterministic printer with a round-trip
  test, and pretty output is a feature for a formatter-in-waiting.

## Evidence

- `prune::tests` (14 tests): drop/transitive-keep/shadowing/generated
  struct names, round-trip, idempotence, and `pruned_programs_elaborate_`
  `identically` — nf and type outputs of original vs pruned are byte-equal
  on the README program, arithmetic, and struct programs.
- `blackbox/cases/lang/prune-*.dpm` (5) + `cases/errors/prune-parse-error.dpm`
  pin CLI bytes and exit codes, all generated from real runs.
- Probes: broken unreachable `let broken : Nat = U;` prunes to exit 0 while
  `nf` on the same program exits 1; empty input exits 1 with the standard
  parse-error shape.

## What would have to be true to revisit

A language feature that lets a declaration affect elaboration without being
*named* at its use sites (instances, coercions, imports) would break the
"unnamed = unreachable" premise; reachability would then need elaborator
hooks. Also, if `prune` output ever needs to preserve user formatting
verbatim, switch to source slices and keep the AST printer as the
normalizing fallback.
