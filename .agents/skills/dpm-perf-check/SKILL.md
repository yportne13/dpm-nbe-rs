---
name: dpm-perf-check
description: Use when a change claims to speed up (or might slow down) dpm-nbe's elaborator, evaluator, or printer — standard workloads, best-of-3 timing discipline, the byte-identical-output oracle, and the known complexity history (the frc strictness fix, the pretty-printer buffer rewrite) that explains what the remaining costs are.
---

# Performance checks for dpm-nbe

## Standard workloads

Use the committed scale cases; they exist so numbers are comparable across
changes:

```sh
for c in scale-num-1000 scale-num-10000 scale-plus-1000-1000 scale-mult-25-25 scale-fib-18 scale-power-2-12; do
  f="blackbox/cases/stress/$c.dpm"
  [ -f "$f" ] || continue
  echo "== $c"; time (target/release/dpm-nbe.exe nf < "$f" > /dev/null)
done
```

Release build only; best-of-3; report the table with before/after.
Reference points on the 2026-10 tree: `scale-num-10000` ≈ 65 ms,
`scale-fib-18` ≈ 49 ms, `scale-power-2-12` ≈ 60 ms (see
`blackbox/findings-stress.md` for the full history).

## The oracle: outputs must not move

A performance change is pure only if every blackbox `.out` sidecar still
passes byte-identical. Run the full suite after rebuilding:

```sh
cargo build --release && blackbox/run.sh
```

If any output changed, either you changed behavior (reclassify the change and
update pinned outputs + golden rows together) or you broke printing (fix it).

## Complexity history — check before optimizing

- **frc strictness (2026-10-25)**: `frc`'s rigid-neutral arms once forced
  whole spines (a mistranslation of Haskell laziness); normal forms were
  O(n²) — 43 s for a 10 000-successor numeral. The whnf short-circuit made it
  near-linear. Do not reintroduce eager forcing in `frc`/`frc_ne`.
  Rationale: [note](../../notes/implemented/2026-10-25-frc-strictness-port.md).
- **pretty printer (2026-10-25)**: `pretty::go` builds strings by appending
  into one caller-owned buffer; it was quadratic via per-node `format!`
  concatenation. Deep application spines are the shape that regresses —
  keep appends single-buffer.
- Remaining measurable cost in NF pipelines is proportional output size
  (quoting + printing a 10 000-node term is inherently O(n)); a timing that
  grows super-linearly in successors beyond that is a regression signal.

## Discipline

- Measure before AND after on the same binary pair; a fresh `cargo build
  --release` for each.
- During exploratory loops use a private `CARGO_TARGET_DIR` so you don't swap
  the shared pinned binary mid-experiment; do the final rebuild + full suite
  once at the end.
- Attribute the phase before editing: `-- type` vs `-- nf` on the same input
  splits elaboration from evaluation+quoting+printing.
