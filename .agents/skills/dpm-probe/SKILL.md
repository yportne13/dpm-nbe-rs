---
name: dpm-probe
description: Use when making or checking any claim about how the dpm-nbe language or CLI behaves — typing rules, error messages, normal forms, exit codes, timings — before writing it into docs, reviews, tests, or commit messages. Probe the binary; never trust source reading or memory alone.
---

# Probing dpm-nbe

The elaborator binary is the only authority. Source reading answers "what
should happen"; probes answer "what does happen". This repo's docs, NOTES.md
deviations, and test tables are all probe-derived — keep that standard.

## Running a probe

```sh
cargo build --release
printf '%s' '<program>' | target/release/dpm-nbe.exe nf    # normal form + type
printf '%s' '<program>' | target/release/dpm-nbe.exe type  # type only
```

- Output shapes: `nf` prints `<nf>\n  :\n<type>`; `type` prints `<type>`;
  user errors go to stderr as `<stdin>:L:C:` + rendered source line + message.
- Exit codes: 0 success, 1 user error (parse/elaboration), 2 CLI/IO misuse,
  101 internal panic. Anything outside {0,1,2} — and any `internal panic` on
  stderr — is a bug, not a result.
- **Never put a literal `λ` in a shell command line** (Git Bash mangles it).
  Write `\ x . t` on the command line; `λ` is safe inside files.
- Long programs: write them to a temp file and redirect (`< file.dpm`), or
  into a blackbox case (see [dpm-blackbox-case](../dpm-blackbox-case/SKILL.md))
  if the probe is worth keeping.
- Error-message text is contract: when a probe result differs from
  `tests/golden.rs` or a blackbox `.err` sidecar, one of them must change in
  the same change set — decide which, don't note it and move on.

## Timing probes

Best-of-3 with the release build only; debug builds are not representative.
Use the existing scale cases as standard workloads:

```sh
time (target/release/dpm-nbe.exe nf < blackbox/cases/stress/scale-num-10000.dpm > /dev/null)
```

Current baselines live in `blackbox/findings-stress.md`; if a timing probe
moves one materially, record it there and follow
[dpm-perf-check](../dpm-perf-check/SKILL.md).

## When a probe surprises you

Trust the binary, re-run once to rule out a mid-rebuild swap, then decide:
semantically wrong → park it (`.skip` + findings entry); surprising but
correct → pin it as a blackbox case so the next person doesn't re-litigate
it. Six such "verified conventions" are recorded in
`blackbox/findings-lang.md`.
