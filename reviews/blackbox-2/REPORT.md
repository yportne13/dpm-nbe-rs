# Black-box adversarial test report — `dpm-nbe` (Rust port)

**Reviewer role:** independent black-box tester. No implementation source was read
(neither `rust/src/**` nor `rust/NOTES.md` nor `rust/tests/**`). Only `README.md`,
the binary and its observable behaviour were used.

---

## 0. Artifact under test, and a reproducibility warning

| item | value |
|---|---|
| binary | `F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe` |
| build A (pinned, all numbers below unless stated) | SHA-256 `33052DDC9B0E113E6BFC57F04822F932CE14A8574B1C63A0EBD1303415D34854`, 417 792 B, 2026-10-14 01:11:14 |
| build B (live mid-session) | SHA-256 `558B7AD56A1550CD1DF0B63589590E229F3856F73D58D3B175600339428DAA4D` |
| build C (live at the very end) | SHA-256 `B925315A761C74606E420588ACABE379B167818276EE5FB0EE90766F73DEFA98` |
| frozen copy used for all results | `bin/pinned-33052DDC.exe` (same hash as build A) |

**The binary was rebuilt twice while this review was running** (392 704 B @ 01:05:07 →
417 792 B @ 01:11:14 → a third build by the end of the session). Everything reported
here was therefore re-run against a *frozen copy* of build A
(`bin/pinned-33052DDC.exe`), and then **the whole 419-probe corpus plus 4 000 fuzz
cases were re-run against build B** (`results_live.jsonl`, `logs_live/`,
`fuzz/fuzz3.*`). Section 2.0 gives the delta; each defect below is tagged with its
status on the two builds (`build A only` / `reproduces on A and B` / `build B only`).

Everything is re-runnable: `python repro.py` regenerates `FINDINGS.txt` from scratch
(`DPM_BIN=<path> DPM_OUT=<file> python repro.py` for another build).

---

## 2.0 Build delta (build A `33052DDC` → build B `558B7AD5`)

28 of 419 probes changed verdict; 0 of the still-succeeding probes changed their
stdout, so the fixes did not disturb normal output.

**Fixed in build B**

| finding | build A | build B |
|---|---|---|
| D1 absurd-lambda panic | exit 101 + panic text | exit 0, prints `λ n (!) . ⊥` (but see N1) |
| D3 `succ` round trip | every successor output unparseable | `succ` is now a keyword, `nf` output re-parses |
| D5 parser depth limit | 512 frames | 1 024 frames (the boundary message bug moved with it) |
| D6 invalid UTF-8 stdin | exit 2 | exit 1, `input is not valid UTF-8` |
| D7 unterminated `{-` | silently accepted | `unterminated block comment`, exit 1 |
| — | `x'` was not a valid identifier | `let x' : Nat = zero; x'` now works |

**Still present in build B** (re-verified directly, see `logs/probe9_live.log`,
`logs/probe10_live.log`, `results_live.jsonl`)

| finding | build B observation |
|---|---|
| **D2 conversion not reflexive** | unchanged: `expecting plus n m equals to plus n m, but it is not` |
| **D4 stack overflow** | unchanged: `0xC00000FD`, now ~31 s wall (0.2 s CPU) |
| D5 boundary message | 1 023 parens OK; **1 024 → `'zero' is a keyword`**; 1 025 → the real message |
| D8 stderr amplification | unchanged: 100 k/1 M identifier → 200 047/2 000 047 B; 10 MB spaces → 20 000 052 B |
| D9 deep `nf` not re-importable | unchanged (now for values ≥ ~1 024 successors) |
| D10 BOM | unchanged: `expected an atom` |
| D11 `_` | unchanged: `expected an atom` / `expected '.'` |
| D12 superlinear normalisation | unchanged |

**Build C (live at the very end of the session, `B925315A`)**

Re-ran the full findings script against it (`FINDINGS_live_C.txt`,
`logs/probe11_C.log`). D1/D3/D6/D7 stay fixed, **but the parser depth limit was
lowered again to 256 frames and the boundary bug is still there** — which shows the
off-by-one is systematic rather than a one-off:

| nested parens | build A | build B | build C |
|---|---|---|---|
| limit − 1 | 511 OK | 1 023 OK | 255 OK |
| **exactly the limit** | 512 → `'zero' is a keyword` | 1 024 → `'zero' is a keyword` | **256 → `'zero' is a keyword`** |
| limit + 1 | 513 → real message | 1 025 → real message | **257 → real message** |

Also unchanged on build C: D2 (conversion), D4 (`0xC00000FD`), D8 (echo),
D9 (deep `nf` not re-importable), D10, D11, D12, and N1 (`λ n (!) . ⊥` still cannot
be re-parsed: `expected end of input`).

**New in build B (still present in C)**

* **N1 — the printed form of an absurd lambda contains `⊥` (U+22A5), which the
  parser rejects.** The D1 panic was replaced by a pretty-printer glyph that is not
  part of the surface syntax:
  ```sh
  printf 'let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C' | dpm-nbe nf
  # exit 0, stdout: 'λ n (!) . ⊥\n  :\nNat → Id Nat (succ zero) zero → Nat\n'
  printf 'λ n (!) . ⊥' | dpm-nbe nf      # exit 1: expected end of input
  printf '⊥'           | dpm-nbe nf      # exit 1: expected an atom
  printf 'let ⊥ : Nat = zero; ⊥' | dpm-nbe nf   # exit 1: expected '_'   <- odd message
  ```
  So absurd-lambda normal forms still cannot be fed back in (the D1 crash became a
  round-trip failure), and `⊥` in a binder position produces the confusing
  `expected '_'`. Either make `⊥` a real token or print the absurd body in a
  re-parseable way.

---

## 1. Executive summary

**Total probes: 419 scripted corpus probes + 194 hand-written probes + 18 000 fuzz cases.**
(≈ 19 000 process executions, every one under a hard timeout; peak RSS and wall time
sampled per probe.)

Verdicts for build A (`33052DDC`):

| verdict | count (419 corpus probes) |
|---|---|
| exit 0 (success) | 158 |
| exit 1 (clean parse/type error) | 248 |
| **Rust panic, exit 101** | **6** |
| **`STATUS_STACK_OVERFLOW` 0xC00000FD** | **2** |
| exit 2 (stdin not UTF-8) | 5 |

**The program can be broken, in four independent ways** (D1–D4). Two of them are
reachable from the README's own example code, one makes the tool's *own output*
unusable as input, one is a hard crash from a 112-byte program. There is also a
class of definitional-equality failures that rejects well-typed programs.
On the newest build (B) only **D2 and D4** are still fully live, plus D5/D8–D12 and
the new N1; D1, D3, D6 and D7 were fixed during the session (section 2.0).

What *does* hold up: the parser is depth-guarded (no stack overflow from parsing,
up to 50 000 nested parens, 1 M-character identifiers, 10 MB inputs); stream
discipline is perfect (no error ever touches stdout, no success ever writes stderr);
`nf`'s type line agrees with `type` mode on all 119 checked well-typed probes;
18 000 fuzz cases produced no crash class beyond the one known panic (and none at all
on build B); panics are deterministic and reproducible.

---

## 2. Confirmed defects

All commands are `printf '<program>' | dpm-nbe <mode>`. Raw bytes for every case are
in `FINDINGS.txt`, `logs/`, and `PROBE_TABLE.md`.

### D1 — `nf` panics (exit 101) on a normal form containing an absurd-pattern lambda — **HIGH** — *build A only, fixed in B (→ N1)*

```sh
printf 'let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C' | dpm-nbe nf
```
```
exit code : 101   (0x00000065)
stdout    : (empty)
stderr    : \nthread 'dpm-nbe' panicked at src\eval.rs:282:20:\n
            eval: impossible (Bot)\n
            note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n
            error: internal panic while elaborating the program\n
```

This is the README's own `C` definition (`λ A (!)`, README lines 72-73), with `C`
instead of `U` as the final term. **`type` mode on the same input is fine** (exit 0),
so the panic is in quoting/printing a value whose body is `Bot`, not in checking.

Wider than the minimal case — all of these panic (exit 101):

| variant | program | mode |
|---|---|---|
| D1b | `let C : (A:U) -> Id Nat (suc zero) zero -> A = \A (!); Id ((A:U) -> Id Nat (suc zero) zero -> A) C C` | nf |
| D1c | `let C : … = \A (!); let x : Id (…) C C = refl; x` | **type** |
| D1d | `let C : … = \A (!); let g : Nat -> (…) = \n . C; g` | nf |
| D1e | `let C : (A:U) -> Nat -> Id Nat (suc zero) zero -> A = \A n (!); C` | nf |
| D1f | two absurd lambdas compared in one type | nf |

Reproducible 3/3, always the same message, always empty stdout. Note the source
location `src\eval.rs:282:20` and the panic text leak to the user; the wrapper's
`error: internal panic …` line suggests panics are caught, but the exit code is
still 101 rather than 1, so scripts cannot distinguish "internal panic" from any
other failure class except by parsing stderr.

*Black-box reading:* an absurd pattern is elaborated to an "impossible" body; the
value is fine until something forces it, and quotation forces it.

### D2 — conversion is not reflexive for stuck primitive (`plus`) neutrals — **HIGH** — *reproduces on A and B*

```sh
printf 'let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f' | dpm-nbe nf
```
```
exit code : 1
stderr    : <stdin>:1:62:
              |
            1 | let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \n m . refl; f
              |                                                              ^
            expecting plus n m equals to plus n m, but it is not
```

The two indices are **byte-identical**, and the checker still says they differ.
This is not restricted to `refl`:

```sh
printf 'let k : (n m : Nat) (p : Id Nat (plus n m) (plus n m)) -> Id Nat (plus n m) (plus n m) = \\n m p . p; k' | dpm-nbe nf
```
```
exit code : 1
            type mismatch
            expected type:
              Id Nat (plus n m) (plus n m)
            inferred type:
              Id Nat (plus n m) (plus n m)
```

A *variable* is rejected against its own type, with two identical types printed as
"expected"/"inferred". Boundary established by probing:

| compared types | result |
|---|---|
| `plus n m` vs `plus n m`, `plus n n` vs itself, `plus n zero` vs itself | **rejected** |
| `g (plus n m)` vs `g (plus n m)` (neutral spine containing a stuck plus) | **rejected** |
| `suc (plus n m)` vs `suc (plus n m)` | **rejected** |
| `plus zero n` vs `plus zero n` (reduces to `n`) | accepted |
| `plus (suc zero) m` vs `suc m` (reduces to `suc m`) | accepted |
| `g n` vs `g n` (plain variable neutral) | accepted |
| `n` vs `n`, `suc n` vs `suc n` | accepted |
| `A -> B` vs `A -> B`, `(x:A) -> B x` vs itself, `Id Nat n m` vs itself | accepted |

So: **evaluation is fine** (`plus (suc n) m` prints as `succ (plus n m)` — see
`probe7.py` section A), but any *comparison* of a type containing a **stuck
`plus`** fails, including with itself. Impact: well-typed programs are rejected,
which for a dependent-pattern-matching/NBE project is a core-property failure, and
the diagnostic is self-contradictory, so users cannot tell a real mismatch from
this bug. *Hypothesis (black box):* stuck primitive applications carry something
(normalised substitution / spine representation) that the equality routine compares
syntactically instead of up to conversion.

### D3 — printer writes `succ`, parser only accepts `suc`: `nf` output is not valid input — **HIGH** — *build A only, fixed in B*

```sh
printf 'suc (suc zero)' | dpm-nbe nf      # exit 0
#   succ (succ zero)
#     :
#   Nat
printf 'succ (succ zero)' | dpm-nbe nf    # exit 1
#   <stdin>:1:1:  …  unbound variable: succ
```

The README documents *both* spellings: the syntax table says `suc t` (line 59) while
the example output is `succ (succ zero)` (lines 27-30). Consequences:

* 41 of 116 re-checked well-typed `nf` outputs fail the raw round trip
  (`crosscheck.json`); **every** output containing a successor fails, including
  printed *types* (`Id Nat (succ (succ zero)) …`).
* `succ` is a legal user identifier (`let succ : Nat = suc zero; succ` → exit 0), so
  the printed constructor name is shadowable — re-importing printed output in a
  context that defines `succ` would silently mean something else.
* After rescuing only `succ`→`suc` and wrapping the term as
  `let v : <printed type> = <printed nf>; v`, **all 48 well-typed seeds round-trip
  byte-identically** (`diffcheck2.json`), so the printer is otherwise faithful —
  this is a pure name mismatch.

### D4 — `STATUS_STACK_OVERFLOW` from a 112-byte program, with a ~25 s stall — **HIGH** — *reproduces on A and B (~31 s on B)*

```sh
printf 'let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (d (d (d (d (d (d (d (d (d (suc zero)))))))))))))))))' | dpm-nbe nf
```
```
exit code : 3221225725  (0xC00000FD = STATUS_STACK_OVERFLOW)
stdout    : (empty)
stderr    : \nthread 'dpm-nbe' has overflowed its stack\n
wall time : ~25-27 s   (CPU time: 0.2 s)
peak RSS  : 75 MB
```

Measured growth of the same shape (`d (d … (suc zero))`, k doublings → 2^k successors):

| k | value | wall | peak RSS | exit |
|---|---|---|---|---|
| 12 | 2^12 | 21 ms | 1.8 MB | 0 |
| 14 | 2^14 | 80 ms | 17.8 MB | 0 |
| 16 | 2^16 | 2 989 ms | 50.8 MB | 0 |
| **17** | **2^17** | **26 921 ms** | **75.3 MB** | **0xC00000FD** |
| 18 | 2^18 | 26 241 ms | 83.4 MB | 0xC00000FD |

Two problems in one: (a) a crash instead of a clean "too deep" error, and (b) the
process burns ~25 s of wall clock with ~0.2 s of CPU before dying — i.e. a trivial
112-byte input stalls a caller for 25 seconds and then aborts. (With a 20 s timeout
it looks like a hang; it is not, it just dies very slowly.) Recursion depth tracks
the *value* size, not the input size.

### D5 — depth guard reports the wrong error exactly at the boundary — **LOW** — *reproduces on A and B (512 on A, 1 024 on B)*

| nested parens | build A | build B |
|---|---|---|
| limit − 1 (511 / 1 023) | exit 0 | exit 0 |
| **exactly the limit (512 / 1 024)** | exit 1, `'zero' is a keyword` | exit 1, `'zero' is a keyword` |
| limit + 1 (513 / 1 025) and above | exit 1, `expression nesting is too deep` | exit 1, `expression nesting is too deep` |

The limit is 512 parser frames on build A and 1 024 on build B (build B: top-level
`->` chains succeed to 2 000, fail at 2 048). At *exactly* the limit the parser
mis-diagnoses the atom `zero` as a binder name, producing a message that has nothing
to do with the real problem; the guard only produces its real message one frame
later. The limit is undocumented, and the error output grows with the caret column
(512–1 024-character source lines are echoed in full).

### D6 — stdin decoding errors exit 2, everything else exits 1 — **LOW** — *build A only, fixed in B*

```sh
printf 'zero\xff' | dpm-nbe nf    # exit 2
# error: cannot read stdin: stream did not contain valid UTF-8
```
README says errors exit 1. Exit 2 is arguably fine, but it is a second, undocumented
error class; `--help` with invalid UTF-8 on stdin still exits 0 (stdin is not read
for usage paths — verified with 5 MB of input in 13 ms).

### D7 — unterminated block comment is silently accepted — **LOW** — *build A only, fixed in B*

```sh
printf 'zero {- oops' | dpm-nbe nf   # exit 0, prints: zero : Nat
printf '{- oops'      | dpm-nbe nf   # exit 1, expected an atom
```
A comment that never closes swallows the rest of the input without any diagnostic if
a term was already parsed. `-}` alone is (correctly) an error.

### D8 — error renderer echoes the entire source line, so stderr ≥ the whole line — **LOW** — *reproduces on A and B*

```sh
printf ' ' x10000000 | dpm-nbe nf    # 10 MB of spaces
# exit 1, stderr = 20 000 052 bytes, 104 ms, peak RSS 58 MB
printf 'a' x1000000 | dpm-nbe nf     # 1 MB identifier, error at column 1
# exit 1, stderr = 2 000 047 bytes
```
The diagnostic prints the line up to the caret *and* a caret-alignment line, so a
single-line 10 MB input yields 20 MB of stderr (and a 100 k/1 M-character identifier
yields 200 047 / 2 000 047 bytes). A cap (e.g. a ±80-character window around the
caret) would remove the amplification.

### D9 — printed normal forms deeper than the parser limit cannot be re-imported — **MEDIUM** — *reproduces on A and B*

`nf` of the k=10 doubling program is a 1 024-deep `succ` tower (~7 KB, one line).
Feeding that text back fails with `expression nesting is too deep` on build A
(limit 512) and *still* fails on build B (limit 1 024 — the printed form needs 1 024
frames for the successors plus one for the wrapper). So even after fixing D3, the
tool's output is not always valid input. (Same for the printed *type* of such a
value.) Any value with more than `limit − 1` successors has this property.

### D10–D13 — smaller items

| id | behaviour | repro | exit |
|---|---|---|---|
| D10 | UTF-8 BOM not skipped → `expected an atom` at 1:1 | `\ufeffzero` | 1 |
| D11 | `_` is neither a binder nor a term; message is `expected an atom` | `let x : Nat = zero; _` | 1 |
| D11b | `_` as a lambda binder → `expected '.'` (points at `_`) | `let f : (A:U) -> A -> A = \A _ . A; f` | 1 |
| D12 | normalisation/printing strongly superlinear (2^14 → 80 ms, 2^16 → 3.0 s, 50 MB) | k=16 doubling | 0 |
| D13 | `Too many patterns.` does not state how many are allowed | `let f : Nat -> Nat -> Nat = \p0 p1 p2 . p0; f` | 1 |

### Confusing / misleading diagnostics (collected)

1. `'zero' is a keyword` — at nesting depth exactly 512 (build A) / 1 024 (build B) (D5).
2. `expecting plus n m equals to plus n m, but it is not` — self-contradictory (D2).
3. `type mismatch … expected: Id Nat (plus n m) (plus n m) / inferred: Id Nat (plus n m) (plus n m)` (D2b).
4. `expected an atom` for the wildcard-looking token `_` (D11).
5. `expected '.'` when the problem is an unsupported `_` binder (D11b).
6. `expected end of input` for `zero;` — never says what was found.
7. `expected end of input` at a NUL byte, with the NUL echoed raw.
8. `Too many patterns.` without the expected count (D13).
9. `unexpected keyword 'let'` at line 2 for a valid program followed by more text.
10. Exit code 2 vs 1 for the two error classes (D6, build A only).
11. `expected '_'` for `let ⊥ : Nat = zero; ⊥` — the message names a character that
    is not in the input (build B, N1).

Positive counter-examples (good diagnostics): `I am not sure if there should be a
pattern '(!)' for equation between plus x zero and succ x.` (conservative and
explanatory), `expression nesting is too deep`, `unbound variable: succ`,
`'suc' is a keyword`, `unterminated block comment` (build B), and the multi-line
`type mismatch` renderer.

---

## 3. Hardening suggestions (not defects)

1. **Never abort with a Windows status code.** Convert the stack overflow into a
   resource-limit error (exit 1) and bound evaluator/quoter recursion (explicit
   work-list, or a bigger worker-thread stack plus a depth counter). If a panic is
   unavoidable, map it to exit 1 and drop the internal `src\eval.rs:282:20` line.
2. **Decide on one successor spelling.** Either accept `succ` as an alias for `suc`
   or print `suc`; then add a round-trip test to the suite:
   `nf(nf(P)) == nf(P)` for every golden case. Also consider making `succ` a
   reserved word so printed output cannot be shadowed.
3. **Add a reflexive-conversion regression test** for every primitive:
   `Id Nat (plus n m) (plus n m)` must typecheck with `refl`; likewise with a
   variable of that type. (This is the cheapest possible guard for D2.)
4. **Cap the echoed source line** in diagnostics (window around the caret).
5. **Raise or bypass the 512-frame parser limit for printed output**, or print deep
   normal forms in a non-nested way; document the limit either way.
6. **Report unterminated `{-`** at EOF (and unbalanced `{- … -}` nesting).
7. **Skip a leading UTF-8 BOM** (and reject a mid-file BOM with a clear message).
8. **Unify exit codes** (pick 1 for all input/elaboration errors, or document 2).
9. **Support `_`** as an ignored binder/term, or make the message explicit:
   "`_` is not a valid binder here".
10. **Make `nf` output self-contained.** 15 of 49 well-typed seeds print a bare
    `refl`/`λ`, which cannot be re-checked standalone ("Can't infer type…").
    Printing `let v : <type> = <term>; v` would make the output always valid input.
11. **Performance:** 2^16 successors take 3 s and 50 MB. If that is quadratic string
    building in quotation/printing, an incremental `fmt::Write` (or a size cap with
    a clear message) would make large-but-legitimate normal forms practical.
12. **`Too many patterns.`** should say how many patterns the expected type allows.
13. **Keep printer output inside the grammar.** The `⊥` glyph printed for absurd
    bodies (N1) is not accepted by the parser, so the round trip is still broken for
    that case; make `⊥` a token or print a re-parseable form.
14. **Fix the off-by-one at the depth limit** (build B: 1 023 parens OK, 1 024 →
    `'zero' is a keyword`, 1 025 → the real message) — the guard should produce the
    depth message from the first rejected frame.

---

## 4. What could not be broken (robustness evidence)

* **Parser depth:** 100/500/511 nested parens, 512-arrow chains, 10 000-deep flat
  application chains (parsed, then a clean type error), 1 000-pattern lambdas (with
  a grouped binder), 50 000 nested parens, `suc` towers of 100 000, 100 k/1 M-character
  identifiers (1 M binder + reference: exit 0 in 30 ms, 15.2 MB), 200 000 lets, 10 MB
  of whitespace — **every one exits 0 or 1, never a crash or a timeout**. The depth
  guard is the reason; it is effective even where the input is huge. Raising it
  (512 → 1 024 on build B) did not reintroduce a parser crash: 300 000 nested parens
  still produce a clean `expression nesting is too deep`.
* **Stream discipline:** 0/419 probes wrote to stdout on failure and 0 wrote to
  stderr on success (`crosscheck.json`).
* **Mode agreement:** for all 119 corpus probes that succeed in `nf` mode, the type
  line printed by `nf` is byte-identical to the whole output of `type`
  (0 mismatches).
* **Round-trip (after the `succ` rescue):** 48/48 well-typed seeds reproduce their
  full `nf` output byte-for-byte when re-checked as
  `let v : <printed type> = <printed nf>; v`; the only 2 exceptions are the depth
  limit of D9.
* **Fuzzing:** 6 000 grammar/mutation cases (`fuzz/fuzz1.*`) + 8 000 cases seeded
  with every anomalous input found by hand (`fuzz/fuzz2.*`, `seeds/`) on build A,
  plus 4 000 more against build B (`fuzz/fuzz3.*`) — 18 000 cases, 0 timeouts,
  0 stack overflows, 0 aborts. On build A there was exactly **one** distinct crash
  class (D1, hit twice); **on build B the 4 000-case round produced zero findings**.
  All other malformed input produced a clean exit 1.
* **No UTF-8 boundary panic:** 2 000-character lines of Greek/CJK/emoji/combining
  characters, ZWJ/ZWNJ runs, BOM placement, NUL bytes, lone surrogates, overlong
  encodings and invalid bytes all produce clean errors; the renderer never slices a
  code point in half.
* **Determinism:** the panic reproduces 3/3; repeated identical probes give
  identical bytes.
* **CLI:** no args, `--help`, `-h`, `help`, `--version`, `-V`, `NF`, `Type`, `--nf`,
  `--HELP`, `--help=1`, `nf extra`, `nf nf nf`, `--`, `-`, `-x`, empty argument,
  space argument, `λ`, `nf\n`, `nf\t`, `nf -h`, `h` → **all print the 210-byte usage
  to stdout and exit 0**, matching the documented contract; usage paths do not read
  stdin (5 MB of stdin → 13 ms). `nf`/`type` are case-sensitive.

---

## 5. Coverage and probe counts

| category | corpus probes | what it covers |
|---|---|---|
| parser | 225 | keywords vs identifiers (`let/letx`, `suc/sucx`, `U/Ux`, `in`, `refl`, `Nat`, `zero`, `plus`, `Id`, rebinding primitives), `_` binders, empty parens/lambdas, `\ x`, `\ x .`, `λ` with no patterns, `(!)` in odd positions, `(--)`, unterminated `{-`, `-}` alone, `->` vs `→` mixing, `--` at EOF, CRLF/LF/CR/tabs/form-feed, empty input, BOM (leading/mid/trailing), non-ASCII & digit identifiers, 100 k/1 M-character identifiers, missing trailing newline, NUL bytes, invalid UTF-8 |
| depth | 58 | nested parens 100/500/512/513/1000/5000/10000/50000, nested lambdas, `suc` towers to 100 000, `plus` towers, `Id` nesting, `->` chains to 50 000, 10 000-atom application chains, 5 000-let programs, deep shadowing (limit 512 on A, 1 024 on B) |
| semantic | 51 | large `plus`, chained `transN` (n = 1…20), README `H`, big lambda normal forms, many simultaneous/dependent lets, conversion corners, absurd patterns |
| cli | 52 | flags combined/duplicated/unknown, `--help` with stdin, empty/unicode args, exit codes per error class, 5 MB and 10 MB stdin, stream separation |
| blowup | 10 | doubling chains 2^10…2^18, `refl` between computed towers, 500-deep towers |
| regress | 23 | one minimal reproducer per finding in this report |
| **hand probes** | **194** | `probe3` 59, `probe5` 49, `probe6` 31, `probe7` 28, `probe8` 20, `probe4` 7 |
| **fuzz** | **18 000** | 6 000 grammar soup + 8 000 mutation of 31 anomaly seeds (build A) + 4 000 (build B) |

Timeouts: every probe had one (10 s default, up to 120 s for the blow-up probes);
nothing was left running.

---

## 6. Raw artefacts (all under `rust/reviews/blackbox-2/`)

| file | contents |
|---|---|
| `REPORT.md` | this report |
| `FINDINGS.txt` | build A: every finding reproduced from scratch: command, exit code, stdout, stderr |
| `FINDINGS_live.txt`, `FINDINGS_live_C.txt` | the same against build B (`558B7AD5…`) and build C (`B925315A…`) |
| `PROBE_TABLE.md` | all 419 probes on build A: id, command, stdin, exit code, stdout, stderr, verdict, note |
| `results.jsonl` | build A, full machine-readable results (33 MB: includes raw stdout/stderr) |
| `results_live.jsonl` | build B, the same 419 probes |
| `logs/<id>.out` / `.err`, `logs_live/<id>.*` | raw bytes of every corpus probe, per build |
| `logs/probe{3,4,5,6,7,8}.log`, `logs/probe9_live.log`, `logs/probe10_live.log`, `logs/probe11_C.log` | console logs of the hand-probe rounds |
| `corpus/`, `manifest.json` | probe inputs + manifest (regenerate with `gen_corpus.py`) |
| `seeds/` | 31 anomaly seeds used by fuzz round 2 |
| `fuzz/fuzz1.*`, `fuzz/fuzz2.*`, `fuzz/fuzz3.*` | 18 000 fuzz results + unique findings; `fuzz/crashes/` has the inputs |
| `bin/pinned-33052DDC.exe` | frozen build A (the binary all detailed numbers refer to) |
| `harness.py`, `fuzz.py`, `diffcheck.py`, `diffcheck2.py`, `crosscheck.py`, `repro.py`, `gen_corpus.py`, `mktable.py`, `analyze.py`, `tail.py`, `make_seeds.py`, `delta.py`, `tag_report.py`, `probe*.py`, `explore*.py` | tooling (standalone; `explore*.py` are the earliest, pre-rebuild scratch probes) |
| `diffcheck2.json`, `crosscheck.json`, `diffcheck.json` | differential-check results |

### Minimal reproducers for the headline defects (status = build B)

```sh
# D2 conversion not reflexive  -- STILL BROKEN on build B
printf 'let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f' | dpm-nbe nf   # exit 1

# D4 stack overflow            -- STILL BROKEN on build B (~31 s, then 0xC00000FD)
printf 'let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (d (d (d (d (d (d (d (d (d (suc zero)))))))))))))))))' | dpm-nbe nf

# N1 absurd body prints an unparseable glyph -- build B only
printf 'let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C' | dpm-nbe nf   # 'λ n (!) . ⊥'
printf 'λ n (!) . ⊥' | dpm-nbe nf                                                 # exit 1: expected end of input

# D1 panic / D3 succ mismatch / D6 exit 2 / D7 silent comment -- FIXED on build B (build A only)
printf 'let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C' | dpm-nbe nf   # build A: exit 101
printf 'succ zero' | dpm-nbe nf                                                  # build A: exit 1
printf 'zero\\xff' | dpm-nbe nf                                                   # build A: exit 2
printf 'zero {- oops' | dpm-nbe nf                                               # build A: exit 0
```
