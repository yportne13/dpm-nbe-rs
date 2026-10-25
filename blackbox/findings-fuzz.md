# Fuzz findings — randomized CLI robustness

Results of `blackbox/fuzz.mjs`, a seeded (deterministic, mulberry32),
dependency-free harness asserting one invariant:

> `dpm-nbe.exe nf|type < program` never panics — exit status is always
> 0 (ok), 1 (user error) or 2 (CLI/IO misuse); never 101/other, never
> stderr containing `internal panic`, never a hang.

## Run parameters

| parameter   | value                                                              |
|-------------|--------------------------------------------------------------------|
| binary      | `target/release/dpm-nbe.exe`, sha256 `7a919ad63518174b4bf02d32af7468f52e30d90dba8f3ad2d869de305c860e6c` |
| date        | 2026-10-25 (branch worktree, binary hash identical before/after each run — no mid-run rebuild) |
| seeds       | 20261025 (default, pinned in script), then 1 and 44444 as extra sweeps |
| inputs      | 2000 per seed, 6000 total (500 per generator per seed)              |
| mode        | `nf` / `type` per input, drawn from the seeded PRNG                 |
| timeout     | 10 s per input (kill; timeout counted as a hang-bug)                |
| platform    | Windows 10, Git Bash, Node.js                                       |

## Result: zero violations

No exit status outside {0, 1, 2}, no `internal panic` on stderr, no
hangs, no crashes. Exit-status distribution (2000-input runs):

- seed 20261025: `0` x 305, `1` x 1695 (305 accepted programs, incl.
  deep-but-legal nestings; 1695 rejected with ordinary diagnostics)
- seed 1: `0` x 276, `1` x 1724
- seed 44444: `0` x 309, `1` x 1691

No run produced exit 2 — expected: per `src/main.rs` that path is only
reachable on stdin read failure or thread-spawn failure, which stdin
bytes cannot trigger.

`blackbox/fuzz-corpus/` is therefore empty/nonexistent: there are no
repros to keep.

## Performance observations

Wall time ~20-23 s per 2000-input run (~10 ms/input spawn overhead on
this machine). Slowest inputs, all structural `nf` normalizations:

| seed | input | shape | size | time |
|------|-------|-------|------|------|
| 20261025 | 1506 | structural, `nf` | 11380 B | 108 ms |
| 1 | 1822 | structural, `type` | 3993 B | 108 ms |
| 44444 | 1214 | structural, `nf` | 10736 B | 135 ms |

Near-misses (inputs taking > 2 s): **none** across all 6000 inputs —
the maximum observed was 135 ms, comfortably inside the 10 s guard.

## Generator coverage

- **tokenSoup** — random tokens from the language vocabulary (`let in
  data struct where elim U U1 U42 Id refl λ \ . ; : -> ( ) _ (!) =`
  plus identifiers/digits/newlines), with occasional deep single-token
  runs up to 700 repeats.
- **mutation** — 1-8 byte-level mutations (byte flip, random/vocab byte
  insertion, deletion, splice from another valid program, truncation,
  region duplication) over five embedded valid seed programs (two
  containing multi-byte λ so flips yield invalid UTF-8).
- **structural** — balanced-ish random trees of parens/lambdas/Pis/
  lets/elim-spines with random leaf tokens, plus targeted runs sized to
  straddle the measured limits: parens 255/256, lambda ~507, letchain
  ~507, digit literals 254/255, Pi nesting 230-300.
- **byteNoise** — uniform random bytes, random valid UTF-8 (incl. CJK,
  emoji, λ), known-invalid UTF-8 sequences (lone continuation, overlong,
  truncated, out-of-range, encoded surrogate, 0xFF/0xFE), BOM-prefixed
  inputs, CR/CRLF/LF chaos over valid programs, NUL sprinkling.

Invalid UTF-8, BOM, NUL and CR-only inputs all exit 1 with ordinary
diagnostics, as the suite's `cases/errors` encodings cases predict.

## Reproducing

```
node blackbox/fuzz.mjs                     # default seed 20261025, n 2000
node blackbox/fuzz.mjs --n=5000 --seed=7   # bigger sweep, still deterministic
```

Same seed + same n regenerates the identical byte-for-byte input
sequence (verified by double-running and comparing per-input sizes and
generators; only wall-clock timings vary).

## Notes / recommendations (not implemented here)

- Wiring `fuzz.mjs` into `blackbox/run.sh` or CI (e.g. a nightly or
  per-PR job with `--n=2000`, default seed) needs an owner decision:
  ~20 s runtime is cheap enough for CI. Not done to keep this change to
  `fuzz.mjs` + this file.
- Corpus retention: `fuzz-corpus/` should be gitignored (it is
  regenerable and only ever contains repros worth copying into
  `cases/` or `.skip` parking per README conventions once triaged).
