# dpm-nbe — independent black-box test report

* Tester: independent black-box agent (no access to `rust/src/**` or `rust/NOTES.md`).
* Target: `F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe` (392 704 bytes, 2026-10-14 01:05).
* Specs used: `rust/README.md` (language + CLI), `README.md` (intent/examples).
* Artifacts in this directory: `run-all.ps1` (assertion suite), `lib.ps1` (harness),
  `bytes-tests.ps1` + `bytes.log` (byte-level stdin tests), `command-log.txt` (full raw log),
  `test-table.csv`, `results.md` (per-case table), `suite-summary.txt`, `p01..p12` + `probe-logs/` (exploration).

## 1. Method

* Every invocation is a fresh process, stdin written as **raw UTF-8 bytes**, stdout/stderr decoded as
  UTF-8 (verified by hexdump: the arrow is `e2 86 92`, line endings are bare `LF`).
* Every run is bounded by a 10–20 s `WaitForExit` timeout with a forced kill fallback; no run timed out
  (slowest observed: 59 ms).
* Expectations were derived by hand from `README.md`/`rust/README.md` and from arithmetic/combinatorics,
  before looking at the observed output. Cases where the spec is silent carry no pass/fail verdict (`INFO`).

## 2. Totals

| suite | invocations | PASS | FAIL | INFO (no verdict) |
| --- | --- | --- | --- | --- |
| `run-all.ps1` (sections A–G + laws) | 230 | 214 | 10 | 6 |
| `bytes-tests.ps1` (raw byte streams, no pass/fail expectations) | 8 | – | 2 discrepancies | 6 as expected |
| **total** | **238** | | | |

* 230 records over **203 distinct input programs**; exit-code histogram of the suite:
  `0 → 159`, `1 → 69`, `101 → 2`.
* The 10 FAILs collapse to **4 distinct defects** (one defect is caught twice, by the round-trip law and by
  the equality suite; another by two different programs).
* Section map: A basics/typing/comments/unicode (37), B README + generated transitivity family (30),
  C definitional equality (25), D error paths & positions (35), E normalization laws (85 runs:
  round-trip, nf/type agreement, plus table 0..6, let-inlining, determinism, printed-type laws),
  F soundness escapes (13), G pattern-grammar edges (5).

## 3. Definite bugs (class a)

### BUG 1 — `nf` crashes (Rust panic, exit 101) when an absurd-pattern value is evaluated
```
> 'let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C' | dpm-nbe nf
<stdin> stderr:
thread 'dpm-nbe' panicked at src\eval.rs:282:20:
eval: impossible (Bot)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: internal panic while elaborating the program
exit code: 101
```
* This is the README's own `C`, merely *referenced* (no application at all).
* Same crash for `... ; C Nat`, for `let f : Nat -> Id Nat (suc zero) zero -> Nat = λ x (!); f`,
  and for the generic dependent case `let f : (n : Nat) -> Id Nat (suc n) zero -> Nat = λ n (!); f`.
* `type` mode on the same inputs succeeds (`let ... ; C Nat` → `Id Nat (succ zero) zero → Nat`), so the
  crash is in *evaluation/quotation of the Bot term*, not in elaboration or pattern checking.
* Expected: exit 0 and a printed value (or, at worst, a normal `exit 1` diagnostic).
* Why it is a bug: an unrecoverable panic is not one of the documented outcomes, it breaks the
  "errors exit 1" contract, and the failing input is the README's flagship example.
* Severity: **high**. Caught by `B13`, `D30`, `E-law` (indirectly), probes `ab01/ab02/ab17/ab18/ab35/pt04/u07`.

### BUG 2 — printed normal forms are not re-parseable: printer emits `succ`, parser only accepts `suc`
```
> 'plus (suc zero) (suc zero)' | dpm-nbe nf
succ (succ zero)
  :
Nat
> 'let z : Nat = succ (succ zero); z' | dpm-nbe nf
<stdin>:1:15:
1 | let z : Nat = succ (succ zero); z
  |               ^
unbound variable: succ          (exit 1)
```
* `succ` is lexed as an ordinary variable, so the printer's output cannot be fed back — a broken
  round-trip. With `succ`→`suc` substituted the same case passes, which isolates the defect.
* Expected: the normal form printed by `nf` should be a valid program (README implies printing is a
  parseable rendering; the language table lists `suc t`).
* Severity: **medium-high** (any downstream tool that consumes `nf` output breaks; the program's own
  diagnostic messages quote `succ` as well, e.g. `expecting succ zero equals to zero`).
* Caught by `E01a`, `E04a` (and `E01b`/`E04b` pass, isolating the spelling).

### BUG 3 — printer emits freshened names with `'`, which the lexer rejects
```
> '(A : U) -> (A : U) -> A' | dpm-nbe nf
(A : U)(A' : U) → A'
  :
U
> 'let z : U = (A : U)(A' : U) → A'; z' | dpm-nbe nf
<stdin>:1:22:
1 | let z : U = (A : U)(A' : U) → A'; z
  |                      ^
expected ':'                     (exit 1)
```
* The identifier charset is letters+digits only (`a1`, `aB`, `abc123`, `A1` ok; `a'`, `a_b`, `a-b`, `a.b`
  all rejected with `expected ':'`), yet capture-avoiding renaming introduces `A'`. The same happens inside
  lambdas (`λ x x' . x'`).
* Severity: **medium** (same class as BUG 2: output not a valid program).
* Caught by `E07a`, `E07b` (and asserted as expected text in `A32`).

### BUG 4 — partially applied pattern lambdas print the *original* pattern list, disagreeing with the printed type
```
> 'let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n refl refl . refl; H zero zero refl' | dpm-nbe nf
λ m n refl refl . refl
  :
Id Nat zero zero → Id Nat zero zero
```
* The value is `H zero zero`, i.e. a function of **2** arguments, but it prints **4** binders while its type
  has 2 arrows. Re-checking the printed term at the printed type gives `Too many patterns.`
  Same for `H zero` (4 binders vs 3 arrows).
* Expected: `λ refl refl . refl` with type `Id Nat zero zero → Id Nat zero zero` (the already-matched
  `m`, `n` must not be re-abstracted).
* Severity: **medium** (normal-form/pretty-printing correctness; `H` is the README's showcase case, and
  this also breaks the round-trip law).
* Caught by `B29`, `B30`, `E06a`, `E06b`.

### BUG 5 — unreadable stdin exits with code 2 and a differently-shaped message
```
bytes 7a 65 72 6f 20 ff fe  |  dpm-nbe nf
stderr: error: cannot read stdin: stream did not contain valid UTF-8
exit code: 2
```
* Same with a lone `0xFF`. Every other error path exits 1 and uses the `<stdin>:line:col:` + source + caret
  format; this one has neither.
* Severity: **low-medium** (contract violation that breaks `if ($LASTEXITCODE -eq 1)` style scripting).
* Caught by `bytes.log` (`b-nonutf8`, `b-loneff`).

## 4. Spec ambiguities / underspecified behaviour (class b)

1. **`(!)` may not be the first pattern.** `λ (!)` and `λ (!) x` both fail with the misleading
   `expected at least one pattern`, while `λ A (!)` (README) and `λ x (!)` work; `(!)` must be last and
   must be preceded by a variable/`refl` pattern. A body after `(!)` yields `expected ';'`.
   The language table says patterns may be variables, `refl` and `(!)` with no ordering restriction.
   *(INFO: `D31`, `G01`, `G02`; probes `ab03/ab06/ab07/wc10/wc12/wc13`.)*
2. **Nested block comments are not supported.** `{- a {- b -} -} zero` ends the comment at the first `-}`
   and fails at the second with `expected an atom` (no hint about nesting). *(INFO: `A37`, probe `v13`.)*
3. **`_` is a `let`-binder wildcard only.** `let _ : Nat = zero; zero` works, but `; _` is `expected an atom`
   and `λ _ . zero` is `expected at least one pattern`. `let = zero; zero` says `expected '_'`, so `_` is a
   deliberate token; README does not mention it. *(INFO: `G03`; `A35`, `A36`.)*
4. **No eta.** `λ x . g x` is not convertible to a neutral `g` (rejected), while it *is* for a reducible `g`.
   Legitimate (eta/funext is not documented) but worth stating. *(INFO: `C21`; `C22` passes.)*
5. **Duplicate patterns allowed, last binding wins.** `λ x x . x` applied to `1 2` yields `2`. Agda rejects
   duplicate patterns; README is silent. *(Asserted as documented behaviour in `G04`, `A33`.)*

## 5. Correct but confusing (class c)

1. **Application errors point at the application head, not at the offending argument.**
   `suc (suc zero) zero` → `<stdin>:1:1: Expected a function type, instead inferred: Nat` although the extra
   argument sits at column 16; likewise `Nat Nat`, `plus zero zero zero`, `Id Nat zero zero zero`, `U U`.
2. **Generic messages for structural failures**: empty/whitespace/comment-only stdin and an unterminated
   `{-` all report only `expected an atom` (at EOF); `{- a {- b -} -} zero` reports `expected an atom`.
3. **Diagnostics quote the printer's `succ`** (`expecting succ zero equals to zero`), a lexeme the user
   cannot write (see BUG 2).
4. **Tab handling**: `\tzero\tfoo` reports column 2; the caret is placed by character count, so with tab
   expansion in a terminal it lands in the wrong column. Cobwebs only, but it misleads.
5. **with-K assumption is accepted where first-order unification fails**:
   `let f : (m : Nat) -> Id Nat (plus m zero) m -> Nat = λ m refl . zero; f` typechecks,
   although `plus m zero` is stuck on the neutral `m`. Such a function can be *defined* but never *applied*
   (every instantiation I tried is rejected by real conversion checks: `f zero refl` / `f (suc zero) refl`
   for the false equation `m = suc m` both fail), and I could **not** derive any false closed proof.
   Corroborating oddity: in one rejected variant the error prints a self-referential substitution —
   `expecting plus (plus m zero) zero equals to zero` — consistent with the forced equation
   `m := plus m zero` being applied cyclically. README explicitly says "I do with-K here", so this is
   reported as a risk/oddity, not as a proven bug.
6. **BOM / UTF-16 stdin**: a UTF-8 BOM makes the whole program fail at 1:1 (`expected an atom`, with the
   invisible BOM echoed in the source line), and UTF-16LE input reports `expected end of input` at 1:2
   (its NUL bytes are valid UTF-8). Realistic on Windows, where `Out-File`/`Set-Content -Encoding UTF8`
   add a BOM and PS 5.1 `>` writes UTF-16. (`bytes.log`: `b-bom8`, `b-bom8nat`, `b-utf16`, `b-nul`.)
7. **CLI**: no args / `--help` / `-h` / `help` / `--version` / `NF` / `nf extra` all print the usage text on
   stdout and exit 0 — matching the stated contract; note that `-h`, `help`, `--version` are simply
   "unknown", which is harmless but means the usage text is the only help.

## 6. Confirmed correct (no discrepancy)

* Both README programs (`trans, H, C, U` and `trans, cong, H, C, U`) typecheck and print `U : U`.
* 20 generated transitivity proofs of my own design typecheck and reduce to `refl`:
  `trans2..trans5` (3–6 endpoints, 2–5 identity hypotheses) at `Nat`, at `U`, at `Nat → Nat`, and at
  `Id Nat zero zero` (higher-order), including a composed `trans2 ... (trans2 ...) ...`. Negative controls
  (`trans2 Nat zero zero (suc zero) refl refl`, `H (suc zero) (suc zero) refl refl`,
  `H zero (suc zero) refl refl`, `cong ... zero (suc zero) refl`) are all rejected.
* `H`'s hard case behaves as intended: `H zero zero refl refl` ⇒ `refl : Id Nat zero zero`, and the
  wrong instantiations fail with `expecting zero equals to succ zero`.
* Definitional equality: 24/25 asserted cases exactly as derived, including `plus zero x ≡ x` and
  `plus (suc zero) x ≡ suc x` for **neutral** `x` (correct), and *rejection* of
  `λ x . plus x zero ≡ λ x . x` for neutral `x` (no bogus funext), `Nat ≢ Nat → Nat`, `zero ≢ suc zero`.
* `plus` is addition for **all 49 pairs** `m,n ∈ 0..6` (49/49), plus spot checks at 6+6 and 3+3.
* Laws: `nf` is deterministic (3/3) and idempotent on the printed normal form after `succ`→`suc`
  (8/8 in that form); `nf` and `type` agree on the type for all 8 sampled terms (8/8); printed types are
  themselves re-parseable normal forms (3/3); `let`-inlining preserves normal forms (6/6).
* Error handling: 33/35 asserted error cases are exactly as specified, including **line 2/3 positions**
  (`<stdin>:2:15:`, `<stdin>:3:1:`), mid-line carets (`1:34`, `1:17`, `1:21`, `1:5`, `1:6`, `1:4`),
  keyword restrictions (`'Nat'/'zero'/'refl' is a keyword`), the helpful pattern diagnostics
  (`It should not be '(!)' for equation between zero and zero, in fact you should try pattern 'refl'.`,
  `It should not be 'refl' ... try pattern '(!)'`, `Expected an identity type for pattern 'refl'`),
  and `Too many patterns.`
* Soundness: 13/13 attempts to turn the accepted with-K equations into a false closed proof were rejected;
  absurd patterns on inhabited identity types are rejected; no wrong normal form was produced anywhere.
* Robustness: 500 nested `suc`, a 500-step `refl` proof, `plus` of 500-deep numerals, 25 nested
  applications and 12-level `plus` all finish in < 60 ms with no stack overflow.
* Encoding: output is UTF-8 with bare LF line endings; CRLF input is accepted.

## 7. Suggested fixes, in priority order

1. Do not panic on Bot: make `eval` of an absurd branch either unreachable by construction or a clean
   `exit 1` diagnostic with a position (BUG 1).
2. Print `suc` (or accept `succ`), and make the renamed binders lexical (e.g. `A1`/`A0`) — restores the
   round-trip property (BUG 2, BUG 3).
3. Quote only the *remaining* patterns of a partially applied pattern lambda (BUG 4).
4. Return exit 1 with the standard `<stdin>:line:col:` shape for unreadable stdin (BUG 5); consider
   stripping a leading U+FEFF.
5. Give `λ (!)`/`λ _ .` a specific message ("absurd pattern must follow at least one variable pattern"),
   and document the `(!)`-last / `_`-in-`let` rules in `rust/README.md`.
