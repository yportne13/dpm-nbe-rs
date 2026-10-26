---
name: dpm-pre-push-checks
description: Use before pushing or claiming a change to dpm-nbe is done — selects the smallest set of checks that covers the outgoing diff (workspace tests, scoped or full blackbox, fuzz, web/vscode builds) without reflexively running everything; CI owns the exhaustive matrix.
---

# Pre-push checks for dpm-nbe

Run the narrowest evidence that would fail for a regression in the outgoing
diff. CI (`.github/workflows/ci.yml`) owns exhaustive coverage: workspace
tests, full blackbox, fuzz, wasm/web build and smoke, extension bundle and
tests. Report only what you ran.

## Select by touched surface

| diff touches | minimum local evidence |
|---|---|
| `src/` (elaborator semantics) | `cargo test --workspace` + `blackbox/run.sh` (full — output bytes are the oracle) |
| `src/parse.rs` (grammar/limits) | workspace tests + `blackbox/run.sh blackbox/cases/errors` and `stress` (depth/width boundary cases) |
| `src/eval.rs`/`pretty.rs` (evaluation/printing) | workspace tests + full blackbox (`.out`s must be byte-identical unless output legitimately changed) + one timing probe on `scale-num-10000` |
| CLI surface (`main.rs`, exit codes, diagnostics) | full blackbox + `node blackbox/fuzz.mjs` |
| `tests/golden.rs` or blackbox cases | the owning suite scoped to the change + full blackbox once |
| `lsp/` | `cargo test --workspace` (lsp tests ride along) |
| `web/`, `vscode/` | `node web/build.mjs` + the two web smokes / `npm run bundle` + the two extension tests, as applicable |
| `NOTES.md`/`README.md` claims | probe every changed claim ([dpm-probe](../dpm-probe/SKILL.md)) |

## Rules

- The full blackbox suite is ~2 minutes — cheaper than arguing about whether
  it's needed; when in doubt for anything under `src/` or `tests/`, run it.
- Never pin a new expectation without having seen the binary produce it.
- If a check fails for a reason owned by an in-flight sibling edit (dirty
  files you didn't touch), re-run once before concluding; don't "fix" files
  you don't own.
- Report: commands run, counts (N passed / N failed / N skipped), and any
  parked `.skip` cases affected.
