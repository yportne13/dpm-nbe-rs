# Black-box test suite

End-to-end tests that drive the compiled CLI (`target/release/dpm-nbe`,
commands `nf` / `type`, program on stdin) and check stdout, stderr, and the
exit status.  They never look at the internals — the binary is a black box.

## Layout

```
blackbox/
  run.sh                  the runner (bash; GNU timeout optional but used when present)
  fuzz.mjs                randomized no-panic fuzzer (node blackbox/fuzz.mjs [--n=…] [--seed=…])
  cases/
    lang/                 semantic correctness (arithmetic, elim, struct, universes)
    errors/               error paths, diagnostics, positions, encoding, exit codes
    stress/               randomized property tests (with oracles) and size limits
  logs/                   one timestamped log per run (regenerable, not load-bearing)
  findings-<area>.md      bugs found while writing the cases (.skip cases live here)
```

## Adding a case

Create `cases/<area>/<name>.dpm` with the program, then any of the sidecar
files `​.mode` (`nf`|`type`, default `nf`), `.out` (exact stdout, trailing
whitespace ignored; absent = stdout must be empty), `.err` (a literal fragment
that must occur in stderr), `.code` (expected exit status, default `0`),
`.timeout` (seconds, default 10).

Case sources may contain any bytes (UTF-8, λ, BOM); sidecars that need
non-printable bytes are produced by a small generator script kept next to the
cases.

Exit-status contract being tested: `0` success, `1` user error (parse or
elaboration), `2` CLI/IO misuse, `101` internal panic.  **A case that yields
101 or hangs is a bug, never an expectation** — park it as `<name>.dpm.skip`
with an entry in the area's `findings-*.md`.

## Running

```
cargo build --release
blackbox/run.sh                    # everything
blackbox/run.sh blackbox/cases/lang
node blackbox/fuzz.mjs             # randomized no-panic fuzz (deterministic seed)
```

The runner prints PASS/FAIL/SKIP per case, writes a log under `logs/`, and
exits nonzero if anything failed.
