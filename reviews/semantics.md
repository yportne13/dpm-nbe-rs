# Semantic fidelity review — Rust port vs. Haskell original

**Revision 2 — re-verification after the fix round (shared task `task-4`).**

* Reviewer: `semantics-reviewer`. Spec: `F:\projects\Haskell\dpm-nbe\src\*.hs`; port: `rust/src/*.rs`.
* This file replaces the first review (`task-1`); §3 records the status of every finding from it.

## 0. Revision pin, method, environment

**Frozen revision (re-frozen by the Lead):** `rust/target/release/dpm-nbe.exe`
SHA-256 `2B7C4F8AE36BA14CAEA1F43A9B34CBA5057B4F1B13CDFD8A27C99D24A1CA2CD5`, built 02:23:05
(the first freeze was `8A769532…`, built 02:14:57; the delta is message-only, §4.13).
Source hashes at the moment of verification (stable across my build, test and probe runs):

| file | SHA-256 (first 16) | mtime |
| --- | --- | --- |
| `src/syntax.rs` | `A05FF3E716750474` | 02:13:07 |
| `src/subst.rs` | `B3A30ACF7876C062` | 01:38:24 |
| `src/eval.rs` | `6893668675693FC3` | 02:13:07 |
| `src/context.rs` | `95178CFBA7F80C18` | 02:22:49 |
| `src/elab.rs` | `465BB54F1212B5D9` | 01:57:39 |
| `src/parse.rs` | `4306EB7BAA6FAD6F` | 02:11:23 |
| `src/pretty.rs` | `0A482AB1C0F117FA` | 01:06:41 |
| `src/main.rs` | `2BAE0B5E32B7530F` | 01:54:29 |
| `src/lib.rs` | `F4A4DDAF9CE8AD67` | 01:57:39 |
| `NOTES.md` | `08DFBFED8C5E4A9A` | 02:23:01 |
| `README.md` | `18ED3E03C8DA92C4` | 02:14:07 |

* **Method.** There is still **no GHC/runghc/cabal/stack** in this workspace, so all "expected"
  Haskell behaviour is derived by reading `../src/*.hs`; every statement about the Rust port was
  executed against the pinned binary or the library. Where a Haskell behaviour is a divergence or
  panic I say so explicitly.
* **Tests (re-run on this revision):** `cargo test --release` → **113 unit tests + 7 golden tests,
  0 failed**; the golden suite also passes with the newest `tests/golden.rs`.
* **Caveat on `tests/golden.rs`:** it is outside the frozen source set and was modified at
  **02:24:46**, after the re-freeze. Its tables hold **66 success + 20 elaboration-error + 10
  parse-error cases** (96 cases) in 7 tests, and all pass against the pinned binary. All other
  numbers below are from the pinned revision.
* The tree moved several times *during* this review (01:47–02:14). Everything in §4/§5 was
  re-checked after the freeze announcement, against the hashes above.
* **Post-verification drift (unannounced).** Immediately after the re-freeze, `src/lib.rs`
  (02:26:37), `src/subst.rs` (02:26:45), `src/syntax.rs` (02:26:45) and `src/pretty.rs` (02:27:01)
  were modified again, so those four rows of the table above describe the revision *the pinned
  binary was built from*, not the current worktree. The pin is therefore the **binary**
  (`2B7C4F8A…`), which is what every probe in §4/§5 used. If the new edits touch evaluation,
  substitution or pretty-printing, re-run §4 and the `O1` probe before relying on this report for
  the next revision.

## 1. Verdict

**The fix round closed every defect I reported in review #1, and the port is now faithful for the
whole CLI corpus — with one remaining corner-case divergence inside the substitution
representation (O1) that is reachable through the library API but not, as far as I could determine,
through the checker.**

* `F1` (missing `frc` of the pattern domain) is **fixed**; `F4` (unterminated block comment) is
  **fixed**; `F5` (error stream/exit code) is now **documented in full**; `F2`/`F3` are
  **documented** as deviation 11; `F6` was and remains unreachable; `F7`'s documentation defects are
  **fixed**.
* The two deviations added since review #1 are verified and correctly described: **deviation 7**
  (`Val::Bot`) and **deviation 13** (structural conversion of stuck `plus` neutrals).
* The re-freeze delta (identifiers longer than 64 characters are truncated in error messages,
  `context.rs:32-46, 126`) is **message-only**: an 80-character *bound* identifier still resolves,
  and a 100 000-character unbound one yields a 277-byte diagnostic (§4.13). It is documented in
  deviation 9's section (`NOTES.md:200-205`).
* **Still open:** `O1` — the layered `subst_sub`/`lookup_sub` rewrite (deviation 4) prefers the
  *newer* substitution's entry when a variable is mapped by both composed substitutions, whereas
  the original's `IM.union` prefers the *argument* (older) one. Demonstrated through the public API
  (§5.1); no CLI input found that reaches it.
* **Minor / cosmetic open items:** `O2` (deviation 1's wording), `O3` (keyword-failure columns and
  the MAX_DEPTH "~256 parens" nit), `O4` (golden case count is 66/20/10, not 75), `O5` (two
  garbled doc comments).

## 2. What changed structurally since review #1 (still faithful)

| Module | Change since review #1 | Fidelity note |
| --- | --- | --- |
| `syntax.rs` | `Sub` is now a list of `Rc<Layer>` (`syntax.rs:253-307`); `Val::Bot` added (`:326-330`) | New internal representation; semantics differ only per `O1`. `Val::Bot` is documented (deviation 7). |
| `subst.rs` | `lookup_sub` resolves layers on demand (`:24-35`) | Equivalent to the original for disjoint layers (verified); see `O1` for overlapping ones. |
| `eval.rs` | `subst_sub` concatenates layers (`:30-56`); `subst_ne` removed; `capp` dropped its unused arity parameter (`:222`); `conv_ne` gained the `Ne::Plus` case (`:354-359`); `Val::Bot` handled in `frc_s`/`conv`/`quote_val` (`:128,335,417`) | All verified; deviation 13 fixes a genuine non-reflexivity of the original. |
| `context.rs` | `bind`/`define` share `push_binding` (`:113-141`); `Error::render` windows long lines (`:27-30, 48-97`) and truncates long names in messages (`:32-46, 126`) | `push_binding` is behaviour-identical to the two former copies. Windowing/truncation are interface deviations, now documented (deviation 9). |
| `elab.rs` | `check_pm` forces the domain in both pattern branches (`:216`, `:242`) | `F1` fix; matches `Elab.hs:133,145`. |
| `parse.rs` | `MAX_DEPTH = 512` with a sticky `too_deep` marker (`:159-166, 306-333`); guards in `raw`/`atom`/`spine`/`pi`/`lam`; BOM treated as whitespace (`:195`); keyword failures point at the token start (`:273-285, 494-514`) | Guard messages now correct on every path (verified on six pathological inputs); see `O3`. |
| `main.rs` | reads bytes and rejects malformed UTF-8 (`:30-47`); surfaces the panic payload (`:71-80`) | Covered by deviation 9. |
| `lib.rs` | `impl From<ParseError> for Error` (`:29-36`) | Pure plumbing. |

Everything else (data types, forcing, application/pattern matching, conversion, quotation,
unification, the checker, the pretty printer) still matches the original clause by clause; see §7.

## 3. Status of the findings from review #1

| # | Finding | Status |
| --- | --- | --- |
| `F1` | `check_pm` matched the pattern domain unforced → rejected a program the original accepts | **CLOSED (fixed)** at `elab.rs:216`/`:242`; regression test `lib::tests::a_forced_domain_is_matched_after_a_refinement`; repro now prints `U/U` (exit 0). |
| `F2` | `\ reflx . zero`: port reads one identifier, original prefix-matches `refl` + `x` | **CLOSED as a finding; documented** as deviation 11 (`NOTES.md:222-233`). It remains a real behavioural deviation from the original (same text, different term). |
| `F3` | `let letx : Nat = zero; letx`: port backtracks, original commits on `let` | **CLOSED as a finding; documented** as deviation 11. Remains a deviation. |
| `F4` | Unterminated `{-` silently accepted | **CLOSED (fixed)**; `U {- x` now reports `unterminated block comment`, exit 1 (`parse.rs:206-222, 320-326`). Matches `skipBlockComment`'s failure class. |
| `F5` | Elaboration errors also go to stderr/exit 1; item 7 understated | **CLOSED**; deviation 9 (`NOTES.md:192-198`) now covers *all* errors plus malformed UTF-8, and states that the original prints to stdout and exits 0. |
| `F6` | Defensive-only differences (`pretty` `#n` fallback; panic texts) | **CLOSED (unreachable)**; unchanged, still unreachable through the CLI. |
| `F7` | Doc defects: stale counts, deviation-3 wording, wrong "deviation 9" refs for `Bot` | **CLOSED**: NOTES/README now state the current unit-test count (113) and no longer state golden totals; deviation 3 now correctly says the original diverges first; `syntax.rs:328`, `eval.rs:304`, `eval.rs:711` all say deviation 7. `F7.2` (hand-rendered parse errors) remains inherent and is described in the "Rust-specific choices" prose. |

## 4. Verification of the fix round and the new deviations (evidence)

All commands run against the pinned binary; "expected" is from the Haskell source.

1. **F1 repro** — `let A : U = Id Nat zero zero; let F : (x : Nat) -> Id Nat x x -> A -> Nat = λ x refl refl . zero; U`
   → port prints `U` / `U`, exit 0 (was: "Expected an identity type … but got: Id Nat zero zero").
2. **F4** — `U {- x` → `unterminated block comment` at 1:3, exit 1; `U {- oops -}` → `U/U`, exit 0.
3. **F5** — `let bad : Nat = U; U` → stderr, exit 1; a file containing `U\n\xFF\xFE\n` → `input is not valid UTF-8`, exit 1 (`main.rs:35-47`).
4. **F2** — `let f : Nat -> Nat = \ reflx . zero; U` → port `U/U`; Haskell parses `[PRefl, PVar x]` and rejects with "Expected an identity type for pattern 'refl' … Nat". Documented (deviation 11).
5. **F3** — `let letx : Nat = zero; letx` → port `zero`/`Nat`; Haskell commits on the `let` prefix and fails to parse. Documented (deviation 11).
6. **Deviation 7 (`Val::Bot`)** — `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C` → `λ A (!) . ⊥` with type `(A : U) → Id Nat (succ zero) zero → A`. Haskell's `eval` has no `Bot` case (`error "eval: impossible"`, `Eval.hs:151`), so the original panics as soon as such a lambda is forced; the port gives `Bot` a value that forces/quotes/substitutes/converts (`eval.rs:128, 304-307, 335, 417`). Claim verified.
7. **Deviation 13 (`conv_ne` / stuck `plus`)** — `let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \ n m . refl; f` → port prints `λ n m . refl`, exit 0. The original's `convNe` (`Eval.hs:179-184`) has no `NPlus` case, so `conv` on two identical stuck `plus` neutrals is `False` and the program is rejected with the self-contradictory "expecting plus n m equals to plus n m, but it is not". The port's structural comparison (`eval.rs:354-359`) restores reflexivity. Claim verified (Haskell side by source reading).
8. **`MAX_DEPTH` guards** — six pathological inputs (`(`×2000, `suc `×5000, `zero `×5000, `(a : U) `×5000, `(a … : U) -> U`, `\ x … . x`) all now report `expression nesting is too deep`, exit 1. Before the 02:11 fix the same inputs produced misleading furthest-failure messages (`expected a binder`, `unexpected end of input`, `expected '->'`) and the two unit tests asserting the message failed (I observed 107/109 at 02:07). 112/112 at the 02:14 revision, 113/113 after the re-freeze.
9. **Boundaries** (measured): 512 `(a : U)` groups / 512 flat arguments / 512 lambda patterns parse (further failures are elaboration errors); 513 of each → "too deep". `suc (`×170 parses, ×171 → too deep. **256 nested parens already fails** (255 is the maximum); NOTES says "~256".
10. **`succ` reserved word** (deviation 12, reworded) — `let succ : Nat = zero; succ` → parse error (`expected '_'`); `\ succ . zero` → parse error; `let x : Nat = zero; succ x` → typechecks as `succ zero : Nat`. In the original, `succ` is an ordinary identifier, so the first two parse (the first prints `zero`) and the third is an elaboration error (`unbound variable: succ`). NOTES now states this explicitly (`NOTES.md:242-247`), so it is a documented deviation; it is *not* purely additive.
11. **`'` in identifiers** — `let z : U = (A : U) -> (A' : U) -> A'; z` → accepted (the original rejects `A'`), purely additive as documented.
12. **Read-back round trips** — printed numerals (`succ …`) and freshened names (`A'`) parse; README program still checks; `k zero` → `λ y . y` and `H zero zero refl` → `λ refl . refl` match deviation 3's table.
13. **Re-freeze delta: long names in messages** (`context.rs:32-46, 126`, `MAX_NAME_ECHO = 64`) —
    an 80-character *bound* identifier still resolves (`let aaaa… : Nat = zero; aaaa…` → `zero`/`Nat`,
    exit 0), so lookup is unaffected; a 100 000-character *unbound* identifier produces a 277-byte
    diagnostic ending in `…` instead of echoing the whole name. `truncate_name` is called only from
    `Ctx::lookup`'s message. The original echoes the full name; the delta is documented
    (`NOTES.md:200-205`).

## 5. Open findings

### O1 (OPEN — real, corner case) layered substitution composition prefers the wrong entry

* Port: `rust/src/eval.rs:30-56` builds `layers = s2.layers ++ s1.layers` (newest first), and
  `rust/src/subst.rs:24-35` resolves a variable in the **first** layer that mentions it.
* Haskell: `src/Eval.hs:18-26` —
  `subst s2 s1 = Sub { dom = dom s1, cod = c, subs = IM.union (IM.map (subst s2) s) (subs s2) }`.
  `IM.union` is **left-biased toward the entries of `s1`** (the argument substitution), each
  substituted by `s2`. The composition-correct value is `(s1 ∘ s2)(i) = s2(s1(i))`: if `i` is in
  `s1`'s map, `s1[i]` wins and is then substituted by `s2`.
* The port's rule coincides with this whenever the key occurs in only one of the two
  substitutions (verified below, case C) but differs when it occurs in **both**: the port takes
  `s2`'s entry, Haskell takes `subst s2 (s1[i])`.
* Minimal demonstration through the **public library API** (compiled against the pinned source;
  no source changes):

  ```rust
  let s1 = insert_sub(Lvl(2), Lvl(1), Val::Zero);            // {1 -> zero}     : Sub 1 2
  let s2 = insert_sub(Lvl(2), Lvl(1), Val::succ(Val::Zero)); // {1 -> suc zero} : Sub 1 2
  let comp = subst_sub(&s2, &s1);
  lookup_sub(&comp, Lvl(1))                    // port: succ zero   Haskell: subst s2 zero = zero
  frc(&subst_val(&s2, &Val::vsub(Val::vvar(Lvl(1)), s1)))  // port: succ zero   Haskell: zero
  // disjoint keys (s3 = {0 -> U}):
  let comp2 = subst_sub(&s3, &s1);             // lookup 1 = zero, lookup 0 = U  (matches Haskell)
  ```

  Measured output: `A overlap … lookup(1)=succ zero`, `B overlap … = succ zero`,
  `C disjoint lookup(1)=zero lookup(0)=U`.
* **Reachability.** I could not reach it through the CLI: `check_pm` unifies a single equation per
  `refl`/`(!)` pattern and substitutes the remaining equations, so the same level is not solved
  twice, and successive refinements map disjoint levels. It *is* reachable through the public
  `eval::subst_sub` / `eval::subst_val` / `subst::lookup_sub` API, and in principle internally
  whenever a value is substituted twice by substitutions that both mention the same level.
* **Secondary note (same function).** `lookup_sub` applies each newer layer as an individual
  `leaf(layer)` (`subst.rs:12-18, 28-31`), so a nested `Val::Sub` produced by a lookup carries the
  *layer's* `dom`/`cod`, not the composed substitution's. The guard in `subst_sub`
  (`eval.rs:31-33`) is therefore evaluated against a different `dom` than the original's; I found no
  reproducible consequence (the guard is a programmer-error check), but it is a latent difference.
* **Suggested fix:** make `lookup_sub` scan the layer list **oldest-first** (so the argument
  substitution's entry wins and the newer layers are applied to it), or resolve the overlap with
  the original's `IM.union` priority. A unit test with two `insert_sub`s on the same level would pin
  it.

### O2 (OPEN — minor wording) deviation 1 overstates

`NOTES.md:76-78` says "`frc` of any lambda value diverges". That is true for a **bare** lambda
value (`eval` of `Tm::Lam`), which is what the CLI always hits, but a lambda wrapped in a delayed
substitution is forced through `frcS`'s `NLam` arm (`Eval.hs:88`), which calls `match` directly and
returns a blocked lambda without diverging. "Forcing a bare lambda value diverges" would be exact.
(Deviation 2's "once deviation 1 is fixed this becomes observable" is now correctly qualified.)

### O3 (OPEN — minor) two residual nits

* **Keyword failures point at the token start** (`parse.rs:273-285, 494-514`): cosmetic, but it
  changes reported columns relative to the original's furthest-failure positions. Not listed.
* **MAX_DEPTH capacity**: 256 nested parentheses already fail (255 is the maximum), so "~256 nested
  parentheses" (`NOTES.md:285-289`) is one too optimistic.

The BOM acceptance, the 160-character echo window and the 64-character name truncation are now
documented (`NOTES.md:200-205, 242-243`), so they are no longer documentation gaps — they remain
interface deviations from the original (which rejects a BOM, echoes the whole line and prints the
whole name), but documented ones.

### O4 (OPEN — cosmetic) golden case counts

`tests/golden.rs` (02:24:46, after the re-freeze) contains **66 success + 20 error + 10 parse = 96
cases** in 7 tests, not the "75 success cases" figure quoted for it. `NOTES.md`/`README.md` no
longer state a golden total, so there is no stale number left to correct — only the 75 figure
itself is wrong.

### O5 (OPEN — cosmetic) garbled doc comments in the pinned revision

* `lib.rs:73`: `/// Normalize a program and also return its type (read back from a semantic`n/// value, i.e. already a normal form).` — a literal `` `n `` where a line break was intended.
* `eval.rs:219-220`: the doc line "Instantiate a closure with exactly its arguments." appears
  twice.

## 6. Module-by-module fidelity (pinned revision)

| Haskell | Rust | Verdict |
| --- | --- | --- |
| `Syntax.hs` | `syntax.rs` | Faithful; `Spine` is a `Vec`, `Sub` is a layer list (O1), `Val::Bot` added (deviation 7), `lvl2Ix`/`lookupEnv` guarded (deviation 10). |
| `Subst.hs` | `subst.rs` | Faithful for disjoint layers; `lookup_sub` differs on overlaps (O1). |
| `Eval.hs` | `eval.rs` | Faithful except deviations 1–4, 7, 13; `subst_ne` was dead code and was removed. |
| `Context.hs` | `context.rs` | Faithful (`push_binding` refactor); `Error::render` adds line windowing (O3). |
| `Elab.hs` | `elab.rs` | Faithful; deviation 5 (`infer` forces `tty`), deviation 6 (forced pattern domain), deviation 8 (`(!)` hint). |
| `Parse.hs` | `parse.rs` | Same grammar; deviations 10–12 plus BOM/window/keyword-position details (O3). |
| `Pretty.hs` | `pretty.rs` | Faithful; `#n` fallback for an out-of-range index is unreachable defensive code. |
| `Main.hs` | `main.rs` + `lib.rs` | Faithful CLI contract; deviation 9 (stderr/exit 1 for all errors, malformed UTF-8), panic payload surfaced. |

## 7. Categories checked with **no** discrepancy found

Below, "no discrepancy" means no difference beyond the claimed deviations (`NOTES.md` items 1–13)
and the open findings above.

* `unify1` clause order (`VVar/VVar` → `VVar` → `VVar` → `VRefl` → `VZero` → `VSucc`/`VSucc` →
  `VZero`/`VSucc` → `conv`), including the `g-1` level and `insertSub g x v` orientation.
* `check`/`check_pm`/`infer` structure, case order, `wrap_lam`, the `Raw::Refl` fall-through to
  "Can't infer type for refl", and every error string (compared character by character).
* `frc`/`frcS`/`frc_res`, `plus`, `napp`, `capp`, `match_`, `eval`, `conv`/`convSp`/`convNe`
  (apart from the documented `NPlus` addition), `newVars`/`convCl`/`freeClosure`,
  `unSubNe(+S)`, `quote`, `nf`.
* `Context.lookupVar`/`bindCtx`/`defineCtx`/`subst` (including `lvl` staying put under
  substitution).
* `Subst.lookupSub`/`insertSub`/`idSub`/`emptySub` for non-overlapping maps.
* Pretty precedences (`3/2/1/0`), `par`, `fresh`, `goPi`, lambda/let layout, `showVal` name list.
* `SrcPos` handling (the port's extra wrapper around parenthesised atoms is shadowed by the inner
  `Raw`), `num_pvars`, `lvl2ix`.
* CLI contract for successful runs (`nf`, `type`, `--help`, unknown argument) and the
  `displayError` layout (identical bytes for short lines).
* The README programs, `genTransN` 1..12 (and the scaling test to 30), and all 96 current golden
  cases.

## 8. Residual uncertainty and how to re-verify

* No GHC is installed, so `O1`'s Haskell side and the "the original rejects/diverges" claims are
  source-derived. `O1` is nevertheless exact: `IM.union`'s left-bias toward
  `IM.map (subst s2) s` is unambiguous in `Eval.hs:26`.
* To re-verify after any further edit: (1) record `Get-FileHash src/*.rs` and the exe hash;
  (2) `cargo test --release` (expect 113 + 7 on this revision); (3) re-run the §4 probes;
  (4) re-run the `O1` probe (a two-`insert_sub` overlap, as in §5.1).
* `tests/golden.rs` changed after the freeze; re-running `cargo test --release --test golden`
  against the pinned binary passes (5/5), but the file's ownership is outside the frozen source
  set, so treat it as a moving input to the verification, not as part of the pin.
