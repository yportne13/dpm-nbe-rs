# Adversarial robustness & CLI-contract review — `dpm-nbe`

**Reviewer**: `robustness-reviewer` (shared task `task-3`)
**Deliverable**: this file + everything under `rust/reviews/robustness/`
**Binary under test (RE-FROZEN)**: `rust/target/release/dpm-nbe.exe`
`SHA-256 2B7C4F8AE36BA14CAEA1F43A9B34CBA5057B4F1B13CDFD8A27C99D24A1CA2CD5`
(built 2026-10-14 02:23:05; `cargo build --release` confirms the tree is in sync).

Every number and verdict below was re-measured against that exact binary after the
final re-freeze. The source moved six times during the review (the Lead hardened
`main.rs`/`lib.rs`/`parse.rs`/`eval.rs`/`elab.rs` in response to the findings); the
earlier revisions and the defects they carried are recorded in §7.

---

## 1. Executive summary

**Status: 0 open findings. 0 CRITICAL, 0 HIGH, 0 MEDIUM, 0 LOW. All six
attack-surface categories are clean.**

| Was | Status on `2B7C4F8A…` |
|---|---|
| **F1** keyword errors pointed past the token (`suc let` → `1:8`) | ✅ fixed — `1:5`, caret under `let` |
| **F2** UTF-8 BOM rejected | ✅ fixed — BOM-prefixed input exits 0 |
| **F3** error echoed the whole source line (~2× input) | ✅ fixed — source line windowed to ~160 chars with `…` |
| **F3r** `unbound variable: <name>` embedded the whole identifier | ✅ **fixed in this revision** — names are truncated to 64 chars + `…`; the 100 000-char probe now emits **277 bytes** instead of 100 210 |
| **F4** caret for tab-indented lines | **by design** — matches the original (`displayError` prints the raw line; megaparsec counts a tab as one column); cosmetic only |
| **F5** depth budget `MAX_DEPTH = 512` | **accepted/documented** — conservative by design (parser runs on the caller's stack; the CLI's 64 MiB thread is defence in depth); capacity note is in `NOTES.md` |

### Category verdict (re-frozen binary)

| # | Category | Verdict |
|---|---|---|
| 1 | Panics | ✅ **clean** — 0 panics in 245 recorded probes + 7 000 fuzz cases |
| 2 | Stack overflow / hangs | ✅ **clean** — no abort, no timeout; over-budget input → exit 1 in ≤ 60 ms |
| 3 | Exponential / resource blow-up | ✅ **clean** — `transN` n=200 in 252 ms; `plus` of two 500-towers in 24 ms |
| 4 | Malformed / edge input | ✅ **clean** — BOM accepted, all error output bounded (216–371 B worst case) |
| 5 | Exit-code & stream discipline | ✅ **clean** — only 0/1; 0 stdout/stderr mix-ups in 245 probes + 7 000 fuzz cases |
| 6 | Error-position correctness | ✅ **clean** — keyword, type, parse and depth positions all point at the offending token (F4 is a documented match with the original) |

---

## 2. Method

* **Harness** `harness/common.ps1` — one process per probe, hard wall-clock
  timeout enforced by polling `Process.HasExited`/`Kill()`, stdin fed as **raw
  bytes**, stdout/stderr drained asynchronously (no pipe deadlock), wall time and
  max sampled working set recorded, output saved verbatim to
  `artifacts/<probe>.{stdin,stdout,stderr}`. No orphaned `dpm-nbe` process
  remained after any campaign.
* **Drivers**: `run_frozen.ps1` (F1–F5 verification + regression guards),
  `run_c45_6.ps1` (CLI contract, malformed input, positions),
  `run_c1_panics.ps1` (panic candidates), `run_c3_extra.ps1` (multi-byte
  positions, resource scaling), `run_c3_v3.ps1`, `run_c2_depth_v2.ps1`,
  `run_c2_depth.ps1` (earlier revisions), `timing_probe.ps1`, `fuzz.ps1`,
  `summarize.ps1`.
* **Fuzzer** `harness/fuzz.ps1` — three generators: *grammar* (structurally valid
  surface programs), *template* (refl/absurd pattern-matching shapes), *raw*
  (random bytes incl. invalid UTF-8). Any exit code ∉ {0,1}, any panic/abort
  banner, any timeout and any stdout/stderr mix-up is recorded with exact bytes.
* **Fuzz campaigns**: 6 × 7 000 = **42 000 cases**. The two campaigns against the
  frozen/re-frozen binary (seeds 90210, 606060) produced **0 findings**; the only
  campaign that ever found anything was the first one (R1), which found the
  `eval: impossible (Bot)` panic.
* **Probes recorded in JSON (re-frozen)**: **245** — `results_cli_malformed_position.json`
  66, `results_panics.json` 51, `results_extra.json` 17, `results_v3.json` 29,
  `results_v2.json` 35, `results_frozen.json` 39, `results_refrozen_timing.json`
  8 — plus ~25 ad-hoc probes; earlier-revision probes are kept as history (§6).
  Timing-sensitive probes were re-measured on an idle machine (the first pass ran
  concurrently with the fuzzer and inflated wall times by ~30 %).
* No GHC/`runghc`/`cabal`/`stack` in this environment, so the Haskell original
  could not be executed; its source was only read for context (which is how F4's
  by-design status was confirmed).
* Harness scripts are ASCII-only (Windows PowerShell 5.1 reads `.ps1` as ANSI);
  non-ASCII inputs are built at runtime with `[char]`.

---

## 3. F3r resolution (the last open item) — verified

| command | before | re-frozen |
|---|---|---|
| `printf 'a%.0s' {1..100000} \| dpm-nbe nf` | 100 210 B | **277 B** ✅ |
| `printf '9%.0s' {1..50000} \| dpm-nbe nf` | 50 208 B | **277 B** ✅ |
| `('a' * 50000) + ' U'` (error mid-line) | 50 208 B | **277 B** ✅ |
| `{-` + 200 000 × `-` | 216 B | 216 B |
| `let z : Nat = U;` + 200 000 spaces | 263 B | 263 B |
| `zero ` × 100 000 | 305 B | 305 B |

`artifacts/f_long_ident.stderr` (277 bytes on disk, 273 characters):

```
<stdin>:1:1:
  |
1 | aaaa…(windowed to ~160 chars)…aaaa
  | ^
unbound variable: aaaa…(64 chars)…aaaa
```

Both the echoed source line and the interpolated name are now bounded, and the
`…` markers make the truncation obvious. No open finding remains.

---

## 4. By-design items (recorded, no action)

### F4 — tab caret matches the original
`printf 'let ok : Nat = zero;\n\tlet bad : Nat = U;\nU' | dpm-nbe nf` → `2:18`;
the caret line is `  | ` + 17 spaces + `^` (tab counted as one column) while the
echoed line contains the raw tab, so in a terminal (tab stop 8) the caret is
3 columns left of the `U` (`artifacts/c6_tab_col.stderr`). The original's
`displayError` prints the raw line and megaparsec counts a tab as one column, so
the port matches the original — cosmetic only.

### F5 — depth budget accepted and documented
`MAX_DEPTH = 512` is charged on every recursion path (`raw()`, `atom()`, spine
length, pattern count, Pi domains, binders), so one parenthesised level costs two
units. Measured limits (`results_frozen.json`, `results_v4_boundary.json`):

| construct | accepted | rejected (exit 1, ≤ 60 ms) |
|---|---|---|
| nested parens | 255 | 256 |
| `suc` chain | 500 | 511 |
| `->` chain | 500 | 511 |
| nested `let` | 500 (122 ms) | 1000 |
| flat application spine | — | 50 000 / 100 000 |

Rejection is clean, fast and bounded (304–306-byte sticky message; the boundary no
longer reports a misleading keyword error). The trade-off is deliberate and
documented in `NOTES.md`; no action.

---

## 5. Defects found earlier and resolved (all re-verified on `2B7C4F8A…`)

| Was | Now |
|---|---|
| R1 `eval: impossible (Bot)` panic, exit 101, 8 inputs (`nf`/`type`, printing, `Id` index, `conv`, `unify`) | ✅ exit 0, prints `λ A (!) . ⊥` |
| R1/R2 uncatchable stack-overflow abort `0xC00000FD` on `suc` towers ≥ 25 000 / flat spines ≥ 50 000 after ~23–30 s | ✅ exit 1 in 36–56 ms with a 305-byte depth message |
| R1/R2 exponential `transN` (n=20 34.5 s, n≥30 > 120 s) | ✅ n=30 46 ms, n=100 67 ms, n=200 252 ms |
| R2 unterminated `{-` silently accepted (exit 0) | ✅ exit 1, `unterminated block comment` at the `{-` |
| R2 non-UTF-8 stdin exit 2 | ✅ exit 1, `input is not valid UTF-8` at 1:1 |
| R4 conversion of stuck `plus` neutrals | ✅ reflexive conversion verified |
| F1 keyword positions | ✅ `suc let` → `1:5`, `U in` → `1:3`, `plus zero let` → `1:11` |
| F2 UTF-8 BOM | ✅ exit 0 in `nf` and `type`; BOM+CRLF and mid-input BOM also fine |
| F3 source-line echo | ✅ windowed to ~160 chars with `…` |
| F3r interpolated identifier | ✅ truncated to 64 chars + `…` (277-byte worst case) |

Regression guards re-run (all exit 0): 400-deep `suc` tower, 500-deep parens,
`transN` 2/4/6/8/10, README `sym`/`trans`/`cong`/`H`/`C`, absurd-lambda printing
and absurd-lambda-as-`Id`-index, `plus` of two 500-towers (7 011-byte normal
form), 400-binder group applied to 400 arguments, printing a 5-`refl` lambda.

---

## 6. Category detail (re-frozen binary)

### 6.1 Panics — clean
51 targeted panic candidates (every `panic!`/`unreachable!`/`assert!` site in
`eval.rs`, `elab.rs`, `syntax.rs`, `subst.rs`) plus 7 000 fuzz cases: **no panic
banner, no exit 101**. Covers the previously-panicking absurd-lambda family,
`lookup_env`/`lvl2ix`/`level underflow` stress (10-`refl` chains, `transN`,
`Id`-of-`Id`, refinement-then-application), `napp` misuse (`zero zero`, `U U`,
`Nat Nat`, `refl refl`), `check_pm` paths (`(!)`/`refl` on non-`Id`, too many
patterns, `(!)` with a right-hand side) — all clean errors.

### 6.2 Stack overflow / hangs — clean
`suc`×511/512/1000/10 000/25 000/50 000/100 000, `U `×50 000/100 000, `zero `×100 000,
parens×256+, arrows×511+, 1000+ nested lets → **exit 1, 36–56 ms**, no abort.
No probe timed out (harness timeout 5–300 s per case). Legal workloads:
`transN` 200 252 ms, `plus` 500+500 24 ms, 400-binder application 151 ms,
500 nested lets 122 ms, 500-deep arrows 128 ms, 500-deep `suc` 25 ms. Sampled
peak memory ≤ 5.6 MB.

### 6.3 Exponential behaviour — clean
`transN`: n=10 30 ms · 14 30 ms · 16 32 ms · 18 35 ms · 20 39 ms · 24 31 ms ·
30 46 ms · 100 67 ms · 200 252 ms. `plus` of two 500-towers 24 ms (7 011-byte
normal form). Wide binder group (400) applied to 400 args 151 ms. No super-linear
blow-up at any tested size.

### 6.4 Malformed / edge input — clean
Empty stdin, zero bytes, whitespace-only, comment-only (`--`, `{- -}`),
unterminated `{-` (exit 1), **UTF-8 BOM (exit 0)**, NUL byte, UTF-16LE bytes,
invalid UTF-8 (exit 1), lone high bytes, CRLF, bare CR, tabs, vertical tab, form
feed, NBSP (whitespace), emoji, 200 KB line without newline, 100 000-char and
50 000-digit identifiers, `--`+200 000 chars, `sucx`/`letx`, trailing data
(`U junk`, `U )`, `U ->`, `U ;`) — all clean, and every error message is bounded
(worst case 371 B).

### 6.5 Exit-code & stream discipline — clean
Across 245 recorded probes and 7 000 fuzz cases: exit codes are only **0**
(success, stdout only) and **1** (error, stderr only); **0** cases with stdout on
failure or stderr on success. CLI contract holds: `--help`, no args, unknown arg,
`-h`, `--`, two flags, valid flag + extra, empty-string arg, `NF` (wrong case) →
usage on **stdout**, exit 0; `nf`/`type` + valid program → exit 0; parse/type
errors, unbound variables, uninferable `refl`/lambda → stderr, exit 1. The exit-2
path in `main.rs` (stdin read / thread spawn failure) is not reachable from
malformed input.

### 6.6 Error-position correctness — clean
Type errors at line 2/3/5 with column > 1 (`let bad : Nat = U;` → `2:17`, caret on
`U`); CRLF gives the same line/column as LF; multi-byte characters count as one
column (`U → x` → `1:5`; CJK/accented identifiers); keyword errors point at the
keyword start (`suc let` → `1:5`, `U in` → `1:3`, `plus zero let` → `1:11`);
`plus a` on line 3 → `3:7`; `(a` on line 3 → `3:3`; `@` on line 2 → `2:1`; unbound
on line 10 → `10:3`; depth errors report the column where the budget is exhausted
(`(`×256 → `1:257`); a block-comment line before the error does not shift
positions. The only nuance is the tab caret (F4), which matches the original.

---

## 7. Probe inventory & artifacts

| File | Contents |
|---|---|
| `results_frozen.json` | 39 probes: F1–F5 verification + regression guards |
| `results_refrozen_timing.json` | 8 idle-machine timing probes for the final revision |
| `results_cli_malformed_position.json` | 66 probes: CLI contract (23), malformed/edge (33), positions (10) |
| `results_panics.json` | 51 panic candidates + clean-error controls |
| `results_extra.json` | 17 probes: multi-byte positions, resource scaling |
| `results_v3.json` | 29 probes: depth limits, edge input, absurd-lambda guards |
| `results_v2.json` | 35 probes: previously-aborting `suc`/spine/`transN` cases |
| `results_depth.json` (R1), `results_v2_boundary.json` (R2), `results_v2_phase.json` (R2), `results_v3_boundary.json` (R3), `results_v4_boundary.json` (R4) | 93 history probes from earlier revisions |
| `artifacts/<probe>.{stdin,stdout,stderr}` | raw bytes/verbatim text for every probe |
| `artifacts/run_*.log` | console logs of every suite run (exit code, wall time, peak WS) |
| `fuzz/findings.jsonl` | final campaigns: `[]` (no findings) |
| `fuzz/findings_old_revision.jsonl` | R1 campaign: 187 hits, all the same `eval: impossible (Bot)` panic |
| `harness/*.ps1` | harness, drivers (incl. `run_frozen.ps1`), fuzzer, summarizer |

Re-run everything: `pwsh -File harness/run_frozen.ps1`, `run_c45_6.ps1`,
`run_c1_panics.ps1`, `run_c3_extra.ps1`, `run_c3_v3.ps1`, `run_c2_depth_v2.ps1`,
`fuzz.ps1`, then `summarize.ps1`.

---

## 8. Revision history & limitations

| Rev | SHA-256 (short) | Time | Notable state |
|---|---|---|---|
| R1 | `33052DDC…` | 01:11 | panic, `suc`/spine aborts, exponential `transN` |
| R2 | `5F0622D2…` | 01:38 | panic + `transN` fixed; aborts remained |
| R3 | `558B7AD5…` | 01:45 | `atom()`/`spine()` guarded, `MAX_DEPTH = 2048` |
| R4 | `B925315A…` | 01:54 | `MAX_DEPTH = 512`, all paths guarded |
| R5 | `8A769532…` | 02:14 | F1/F2/F3 fixed; F3r residual |
| **R6** | **`2B7C4F8A…`** | **02:23** | **re-frozen: F3r fixed — 0 open findings** |

1. **Pinned to `2B7C4F8A…`.** The fast suite (198 probes) takes ~2 minutes, the
   fuzzer ~3.5 minutes. Every run has a hard timeout and left no orphaned process.
2. **No Haskell toolchain** here, so F4's "matches the original" was confirmed by
   reading `Pretty.hs`/`Parse.hs`, not by executing it. The R1 panic *was* shared
   with the original (`Eval.hs:151`; `convNe`→`convCl`→`capp`).
3. **Memory** is the maximum sampled working set (20 ms poll), a lower bound;
   adequate for the "no blow-up" claim.
4. **Not covered**: process-level signals/console interaction, stdin > 6 MB, and
   `Ctrl-C` behaviour.
5. **No open items.** The review is complete for this revision; any further source
   change should re-run `harness/run_frozen.ps1` (10 s) plus the fuzzer (3.5 min).
