---
name: dpm-blackbox-case
description: Use when adding, running, or triaging black-box CLI test cases in blackbox/ — case format and sidecars, scoped runs, the .skip parking convention, findings files, and the byte-level oracle rules that make pure-performance changes provable.
---

# Black-box cases in blackbox/

The suite drives `target/release/dpm-nbe[.exe]` as a black box and pins
stdout, stderr fragments, and exit codes. Full contract:
[blackbox/README.md](../../../blackbox/README.md).

## Adding a case

Create `cases/<area>/<name>.dpm` (the program; any UTF-8, `λ` included) plus
sidecars as needed:

| sidecar | meaning | default |
|---|---|---|
| `.mode` | `nf` or `type` | `nf` |
| `.out` | exact stdout, trailing whitespace ignored | absent = stdout must be empty |
| `.err` | literal fragment that must occur in stderr | unchecked |
| `.code` | expected exit status | `0` |
| `.timeout` | per-case seconds (GNU timeout) | `10` |

Every `.out`/`.err` must be **generated from an actual run**, never written
from imagination. Probe first ([dpm-probe](../dpm-probe/SKILL.md)), then pin.

Areas: `lang/` semantic correctness, `errors/` diagnostics/positions/encoding,
`stress/` randomized property tests and limits. Read the existing case names
in the target area before adding — duplicates waste suite time.

Files needing raw bytes (BOM, invalid UTF-8, NUL, 100k identifiers) are
produced by the area's generator script (`cases/errors/gen.sh`,
`cases/stress/gen.mjs`) — extend those instead of hand-writing binary files.

## Running

```sh
blackbox/run.sh                        # everything (~2 min)
blackbox/run.sh blackbox/cases/lang    # one area — do this while iterating
```

The runner logs to `blackbox/logs/`, exits nonzero on any failure, and flags
stderr containing `internal panic` regardless of expectations.

## Triage rules

- Failing case that should pass = either your expectation is wrong (probe and
  fix the case) or the binary is wrong (see below).
- Binary wrong (bad semantics, exit 101, panic, hang): rename to
  `<name>.dpm.skip` (sidecars keep their names) and write the repro + judgment
  into that area's `findings-*.md`. Never delete the case; never mark a bug as
  expected with `.code 101`.
- Regenerating stress cases: `node blackbox/cases/stress/gen.mjs` is
  seed-committed and must reproduce byte-identical files; if you change
  templates, re-verify determinism by running it twice and comparing hashes.
- `.out` sidecars are the byte-level oracle: a pure-performance change must
  leave every one of them passing untouched. If a legit output format change
  happens, the golden rows and every affected `.out` change together in one
  change set.
