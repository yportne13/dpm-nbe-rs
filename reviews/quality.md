# Quality review — `rust/` (code quality, readability, idiom, tests)

Reviewer: `quality-reviewer` (task-2). Scope: `rust/src/*.rs`, `rust/tests/golden.rs`,
`rust/README.md`, `rust/NOTES.md`, `rust/Cargo.toml`.
I own only this file; every suggested change below is a snippet, nothing was edited.

---

> ## FINAL VERDICT (supersedes the addendum history that was here)
>
> Verified independently by me, not taken on trust, on the **re-frozen revision**:
> `target/release/dpm-nbe.exe` SHA-256
> `2B7C4F8AE36BA14CAEA1F43A9B34CBA5057B4F1B13CDFD8A27C99D24A1CA2CD5` — I rebuilt the
> release binary from source myself and got exactly this hash. Source hashes
> `sha256[0..16]`: `context.rs 95178CFB`, `elab.rs 465BB54F`, `eval.rs 68936686`,
> `lib.rs F4A4DDAF`, `main.rs 2BAE0B5E`, `parse.rs 4306EB7B`, `pretty.rs 0A482AB1`,
> `subst.rs B3A30ACF`, `syntax.rs A05FF3E7`, `golden.rs F83D9339`, `NOTES.md 08DFBFED`,
> `README.md 18ED3E03`, `Cargo.toml 2311B24D`.
>
> Acceptance, run by me at that revision:
>
> * `cargo clippy --all-targets -- -D warnings` → **exit 0, 0 warnings**.
> * `cargo fmt --check` → **exit 0, clean**.
> * `cargo test` **and** `cargo test --release` → **exit 0**: 113 unit passed, 7 golden
>   passed, 0 failed, 0 ignored, 0 doc-tests.
> * Delta over the previous freeze (identifier truncation in error messages) verified
>   directly: a 100 000-character identifier exits 1 in ~65 ms with **655 B** of stderr —
>   the echoed source line and the message are both windowed with `…`, and there is no
>   abort. The two new golden tests `bom_is_accepted_deviation` and
>   `long_line_errors_are_windowed_deviation` pass.
>
> Freeze-process note (not a defect in the code): the earlier quoted hash `8A769532…` was
> already stale when it was sent — it predated the `context.rs` / `golden.rs` edits — and
> two transient breaks were visible while the last edits were landing
> (`context.rs:110` briefly called an undefined `truncate_name`, `E0425`, so the lib did
> not compile; `tests/golden.rs` briefly carried a non-snake-case test name that failed
> `clippy -D warnings`). Both are fixed here. Because a source edit that lands after the
> release build silently invalidates the binary hash, re-run the three commands and re-hash
> the binary after any further change.
>
> `parse.rs` is unchanged from the revision where I verified M1, so that verification
> carries over.
>
> **M1 (the only MUST FIX) is fixed and verified.** `MAX_DEPTH` is now enforced on every
> path I originally found unguarded — `raw` (`parse.rs:335-343`), `atom` (`:471-478`,
> body renamed `atom_inner`), `spine` (`:455-464`, argument count), `pi` (`:398-411`,
> binder groups) and `lam` (`:359-372`, patterns) — with a sticky marker (`too_deep`,
> `:159-163`, applied at `:308-312`) so the boundary always reports "expression nesting is
> too deep" with a position. Against the hash-matched release binary: `suc `×5000,
> `zero `×20000, 3000 parens, 5000 Π groups and 5000 patterns all exit **1** in 16-60 ms
> with that message (no `0xC0000409`); 400 flat `suc`s, 150 nested `suc (...)`, 400 Π
> groups and 500 chained `let`s still parse. The new
> `parse::tests::every_nesting_path_is_bounded` (`:714`) covers exactly the five paths,
> and `nesting_below_the_limit_is_accepted` (`:731`) pins the accepted side. A 12-input
> robustness sweep (20 k-char identifier, unterminated `{-`, 600 patterns, 600 lambdas,
> invalid UTF-8 on stdin, empty input) produced clean exit-1 errors and no abort.
>
> **Verified closed since the detailed sections below were written:** M0 (build break),
> M2 (fmt), S1 (counts now 113 unit / ~110 golden in `README.md` and `NOTES.md`), S3 (`push_binding`
> at `context.rs:125`; `names` documented as parity-only at `:13`), S5 (`capp(cl, sp)` at
> `eval.rs:222`; `unify_s(_d)` kept deliberately with a comment — accepted), S6 (no
> undocumented `eval.rs` function remains in `missing_docs`), S7 (`let … else`,
> `elab.rs:161`), S8 (payload downcast, `main.rs:74-78`), S9 (`fmt` is now a one-line
> fold, `elab.rs:377-380`), S10 (content), S11 (`NOTES.md:100-105` now states that the
> original diverges before that rule and marks the table unreachable), S12 (byte-exact
> caret assertion, `golden.rs:829-833`), S13 (`lib.rs:548-571` asserts the exact 150-level
> normal form and no longer claims `join` catches a stack overflow), S14.1 (`Bot` =
> deviation 7), S14.2 (`bot_is_a_value`, `eval.rs:709-717`, plus the `lib` regression test
> at `:575`), S15, N1 (`is_keyword` comment), N4 (`From<ParseError>`, `lib.rs:29`), N8
> (README "a subset of"), and the new deviation 13 (`conv_ne` structural `Ne::Plus`,
> `eval.rs:359`): `Id Nat (plus n m) (plus n m)` now checks while `plus n m` vs
> `plus m n` is still rejected — both verified by hand.
>
> **Status on the re-frozen revision**
>
> * **MUST FIX: none.** M0, M1, M2 and every finding from the first pass are closed at this
>   revision (verified above and in the list preceding this block).
> * **SHOULD FIX (2, neither blocks shipping):**
>   * **F1.** `src/lib.rs:73` is a malformed doc comment — one line containing a literal
>     `` `n `` and an embedded `///`:
>     `/// Normalize a program and also return its type (read back from a semantic`n/// value, i.e. already a normal form).`
>     It compiles and `cargo fmt` accepts it, but `cargo doc` renders the escape. Split it:
>     ```rust
>     /// Normalize a program and also return its type: the type is read back from
>     /// its semantic value, i.e. it is already a normal form.
>     ```
>   * **F2 (S4 only half done).** Contrary to the closure note, five of the six dead-API
>     items are still present and still unreferenced (each a single-occurrence definition):
>     `pretty::show_val_closed` (`pretty.rs:149`), `Env::is_empty` (`syntax.rs:216`),
>     `impl Default for Env` (`syntax.rs:221`), `impl fmt::Display for Pn`
>     (`syntax.rs:83`), `subst::empty_sub` (`subst.rs:43`, used only by its own test at
>     `:90-91`); `Raw::strip_pos` (`syntax.rs:127`) is test-only as well. Only `subst_ne`
>     was actually deleted. Delete the rest, make them `pub(crate)` / `#[cfg(test)]`, or
>     say in `NOTES.md` that they are kept for parity with `../src`.
> * **NICE TO HAVE.** `eval.rs:705` and `:719` both carry the `// -- substitution ---`
>   section header, sandwiching the `Bot` test — move or rename one; `pretty::pretty_tm`
>   (`pretty.rs:44`) is still the only public function `missing_docs` flags;
>   `every_nesting_path_is_bounded` asserts `msg.contains("deep")` rather than the exact
>   message; `NOTES.md` §10 lists parens/`suc`/binders but not the fact that the shared
>   512 budget also caps chained top-level `let`s (measured: 500 parse, 600 rejected) —
>   one clause would close that; N2/N5/N7 remain accepted as cosmetic, and the tab-caret
>   alignment matches the original.
>
> **Final verdict: ship it.** No MUST FIX remains. Readability and conciseness are good
> (1:1 module mapping with the Haskell original, WHY-comments, justified `Rc`/`Box`, no
> `unsafe`/`#[allow]`, no `unwrap` on user input); the test suite is genuinely strong
> (113 unit + 7 golden, black-box CLI cases with byte-exact rendering, and the robustness
> boundary, long-line windowing and `Val::Bot` all covered); and the one abort-class defect
> I found is fixed with regression tests. The only open items are a malformed doc comment
> and harmless dead API.

---

## Revision reviewed — read this first: the tree moved and is currently broken

The port was edited by teammates **continuously during this review**
(`eval.rs`: `D2F30CD4` → `8260960C` → `9C644853` → `8E94A928`;
`lib.rs`: `024D9965` → `EA9F0AFD`; `syntax.rs`: `CB4388AC` → `DE271D93` → `D52D34C2`;
`subst.rs`: `B4DA920B` → `C7F7AB00`).

**Last revision at which the whole acceptance suite was green** (every finding below was
verified against it):

| file | sha256[0..16] | lines | file | sha256[0..16] | lines |
| --- | --- | --- | --- | --- | --- |
| `src/lib.rs` | `EA9F0AFD86B5922F` | 524 | `src/pretty.rs` | `0A482AB1C0F117FA` | 260 |
| `src/eval.rs` | `8E94A928010901A2` | 707 | `src/subst.rs` | `B4DA920B22262715` | 71 |
| `src/elab.rs` | `EF243441DB41CD47` | 546 | `src/main.rs` | `FA9E953E51649EC1` | 68 |
| `src/context.rs` | `AC840C3F37194C91` | 146 | `tests/golden.rs` | `596566FD833E6B89` | 904 |
| `src/parse.rs` | `1E5A1F9F4144FB9F` | 634 | `README.md` | `C985D1DE6AEE5474` | 96 |
| `src/syntax.rs` | `DE271D93A6810C45` | 441 | `NOTES.md` | `2C730BE38C18B764` | 196 |

**Live revision at hand-off** — `syntax.rs` `D52D34C209DF770A` (474 lines) and
`subst.rs` `C7F7AB00832904E9` (90 lines) were rewritten to represent a substitution as a
list of `Rc<Layer>` (`Sub { dom, cod, layers }`, `Sub::new/id/empty/raw`, `subst::leaf`)
instead of a single `BTreeMap`. **That refactor is incomplete: the tree does not build.**

```
$ cargo clippy --all-targets -- -D warnings      # same for cargo build / cargo test
error[E0609]: no field `subs` on type `&syntax::Sub`      (x2, lib)
error[E0609]: no field `subs` on type `syntax::Sub`       (x5, lib test)
error: could not compile `dpm-nbe` (lib) due to 2 previous errors
error: could not compile `dpm-nbe` (lib test) due to 7 previous errors
                                          CLIPPY_EXIT=101, TEST_EXIT=101   <-- BROKEN

$ cargo fmt --check
Diff in ...\src\eval.rs:439      (offending line 442, 102 chars)
Diff in ...\src\subst.rs:10      (single-line struct literal, line 13)
Diff in ...\src\syntax.rs:280    (single-line struct literal, line 283)
Diff in ...\src\syntax.rs:287    (single-line struct literal, line 290)
Diff in ...\src\syntax.rs:292    (single-line struct literal, line 295)
                                                    FMT_EXIT=1
```

Consequences for this review: **M0 below is the blocking item right now**; the `Sub`
refactor still needs its callers migrated (e.g. `eval.rs:38-45` reads `s1.subs`, and the
`subst_sub` body is written against the old single-map representation). Line numbers
below are as of each file's own last hash: `eval.rs`, `lib.rs`, `elab.rs`, `context.rs`,
`parse.rs`, `pretty.rs`, `main.rs`, `golden.rs`, `README.md`, `NOTES.md` are unchanged
since the green revision; the `syntax.rs` / `subst.rs` numbers were re-checked at the
live revision (both files only grew around `Sub`, so the cited items below stand).

Toolchain: `rustc 1.87.0-nightly (287487624 2025-02-28)`, edition 2021.
No `unsafe`, no `#[allow(...)]`, no crate-level lint attributes anywhere in `src/`.

## Commands run (exact results, last green revision — see the live block above)

Run from `rust/`; clippy was run after `cargo clean -p dpm-nbe` so it could not replay
cached diagnostics:

```
$ cargo fmt --check
Diff in \\?\F:\projects\Haskell\dpm-nbe\rust\src\eval.rs:439:
-            if consumed < ps.len() && splittable && prefix_matches(&ps[..consumed], &sp[..consumed]) {
+            if consumed < ps.len() && splittable && prefix_matches(&ps[..consumed], &sp[..consumed])
+            {
                                                    FMT_EXIT=1        <-- FAILS

$ cargo clippy --all-targets -- -D warnings
    Checking dpm-nbe v0.1.0 (F:\projects\Haskell\dpm-nbe\rust)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.94s
                                                    CLIPPY_EXIT=0    (0 warnings)

$ cargo test
    Running unittests src\lib.rs  ... 100 tests: 99 passed; 0 failed; 1 ignored
    Running unittests src\main.rs ...   0 passed; 0 failed
    Running tests\golden.rs       ...   5 passed; 0 failed
    Doc-tests dpm_nbe             ...   0 passed; 0 failed
                                                    TEST_EXIT=0
```

`Cargo.lock` contains exactly one package (`dpm-nbe`): "no dependencies" is true.
`cargo build --release` is clean. `src/` = 3 300 lines total, 2 141 before the first
`#[cfg(test)]`, **1 646 non-comment implementation lines**; `tests/golden.rs` = 904.
The Haskell original is 1 087 lines across 8 modules.

## Verdict

**Readability: good.** The module split, type names and control flow mirror
`../src/*.hs` one-to-one, so the port can be read side-by-side with the reference.
Comments overwhelmingly explain *why*: `eval.rs:123-136` (why an empty-spine lambda is
already a value), `eval.rs:213-220` (why `match_` re-runs from the original patterns),
`elab.rs:302-306` (why application forces the inferred type), `eval.rs:22-29` (the
`dom` fix), `syntax.rs:341-344` (`progressed`), `syntax.rs:227-231` (closures stable
under substitution). I found no comment that merely restates code, and none that is
stale with respect to the behaviour it describes.

**Conciseness: good.** 1 646 implementation lines for a faithful port of 1 087 lines of
Haskell is not bloat; the difference is explicit `Box`/`Rc`, `Result` plumbing and a
hand-written parser. The noise is local and listed below (`capp(_n)`, `unify_s(_d)`,
`fmt`, the `Raw::Refl` guard + `unreachable!()`, the test-only counter in `subst_sub`),
not structural.

**Test quality: good, with real gaps.** The unit tests assert behaviour, and the
5-test golden suite is genuinely black-box: it spawns the built binary through
`CARGO_BIN_EXE_dpm-nbe`, pins exact stdout, stderr, exit codes and error positions, and
its expectations were hand-derived from `Pretty.hs`/`Elab.hs` rather than recorded from
the port — I spot-checked several against the Haskell source (e.g.
`(m : Nat)(n : Nat) → ...` from `goPi`; the `"!"`→`"(!)"` deviation from
`Elab.hs:142`) and they hold. The gaps: the robustness boundary in M1 is untested and
the newest deviation (`Val::Bot`, DEVIATION 9) has **no test at all** and **no NOTES
entry** (S14), and one `#[ignore]`d diagnostic test is the sole reader of test-only
instrumentation injected into `subst_sub` (S15).

**Three MUST FIX:** the tree does not compile at the live revision (M0, a mid-flight
`Sub` refactor); the `MAX_DEPTH` guard does not cover the parser's `atom()`/spine
recursion, so a ~20 KB stdin payload aborts the process with a stack overflow
(exit `0xC0000409`) instead of the documented error format (M1); and `cargo fmt --check`
fails in five places (M2).

---

## MUST FIX

### M0. The tree does not compile at the live revision (blocking)
`cargo build`, `cargo clippy --all-targets -- -D warnings` and `cargo test` all fail with
7 × `error[E0609]: no field 'subs' on type 'Sub'` (`CLIPPY_EXIT=101`, `TEST_EXIT=101`).
`Sub` is now `{ dom, cod, layers: Vec<Rc<Layer>> }` (`src/syntax.rs:274-278`) with
`Sub::new/id/empty/raw`, but callers still use the removed single-map field — the first
being `eval::subst_sub` (`src/eval.rs:30-46`, which iterates `s1.subs` / `s2.subs`) and
its tests. Finish the migration (or revert the refactor) before anything else; no other
acceptance check can run until then.

### M1. `MAX_DEPTH` only guards `raw()`; `atom()`/`spine()` recursion aborts the process

`parse::MAX_DEPTH` (`src/parse.rs:53-55`) is consulted only in `raw()`
(`src/parse.rs:265-275`). Two unbounded recursion paths bypass it:

* `atom()` recurses into itself for the arguments of `suc`/`plus`/`Id`
  (`src/parse.rs:400-406`) without touching `self.depth`;
* `spine()` (`src/parse.rs:374-380`) is a loop, so it builds a left-nested
  `Raw::App` chain of unbounded depth.

The elaborator (`check`/`infer`) and `eval`/`quote_val` then recurse over the
resulting term with the same depth, so the 64 MiB CLI thread (`src/main.rs:39-45`,
`src/lib.rs:44`) only raises the bar.

Reproduced against `target/debug/dpm-nbe.exe` (stdin, `type` mode) at the last green
revision; `src/parse.rs` is unchanged since (`1E5A1F9F4144FB9F`), so the repro still
applies:

| input | result |
| --- | --- |
| `suc `×4500 + `zero` | exit 0, prints `Nat` |
| `suc `×5000 + `zero` | **exit `-1073741571` (0xC0000409)**, stderr `thread 'dpm-nbe' has overflowed its stack` |
| `zero `×5000 | exit 1 (normal type error) |
| `zero `×20000 | **exit 0xC0000409**, same stack-overflow message |
| `(`×3000 + `U` + `)`×3000 | exit 1, `<stdin>:1:513:` + `expression nesting is too deep` (guard works for this path) |

A ~20 KB input kills the CLI with a raw abort code, on a program whose stated contract
is "errors are printed to stderr (…) and exit with status 1" (`README.md:32-33`) and
whose `NOTES.md:160-166` claims `MAX_DEPTH` prevents the stack overflow.
`NOTES.md:167-168` ("`lookup_env`/`lvl2ix` are programmer-error paths") also understates
the reachable failure set.

Minimal fix — count `atom` recursion against the same budget:

```rust
fn atom(&mut self) -> PResult<Raw> {
    self.skip_ws();
    if self.depth >= MAX_DEPTH {
        return self.fail("expression nesting is too deep");
    }
    self.depth += 1;
    let r = self.atom_inner();
    self.depth -= 1;
    r
}
```

(rename the current body to `atom_inner`), and also count each `spine()` iteration
(or bound the spine length), because the long-application path is independent of
`atom`. With both bounded, term depth is bounded by `MAX_DEPTH`, the elaborator
recursion follows, and the big CLI thread becomes defence in depth.

Tests to add: `parse::tests::deeply_nested_input_is_rejected_without_overflowing`
(`src/parse.rs:602-606`) only covers `(`-nesting; add a `suc`-chain and a long-spine
case, plus one CLI/golden case asserting the "too deep" message and exit 1.

### M2. `cargo fmt --check` fails (5 diffs at the live revision)

```
Diff in ...\src\eval.rs:439      -            if consumed < ps.len() && splittable && prefix_matches(&ps[..consumed], &sp[..consumed]) {
Diff in ...\src\subst.rs:10      -    Sub { dom: layer.dom, cod: layer.cod, layers: vec![layer.clone()] }
Diff in ...\src\syntax.rs:280    -        Sub { dom, cod, layers: vec![Rc::new(Layer { dom, cod, subs })] }
Diff in ...\src\syntax.rs:287    -        Sub { dom: n, cod: n, layers: Vec::new() }
Diff in ...\src\syntax.rs:292    -        Sub { dom: d, cod: Lvl::ZERO, layers: Vec::new() }
```

`src/eval.rs:442` is 102 characters (inside the new `quote_ne`); the other four are
single-line struct literals that rustfmt wants expanded. `cargo fmt` fixes all five.
At the last green revision the tree was fmt-clean except for the `eval.rs` line, so this
is a fresh regression from the `quote_ne` and `Sub` edits, and it blocks the stated
acceptance criteria.

---

## SHOULD FIX

### S1. Stale test counts in both docs
`README.md:8` and `NOTES.md:185` say "98 unit tests"; the suite now collects **100**
(99 passed + 1 ignored, `diag_trans_scaling`). `README.md:8`'s "(75 golden cases)"
counts only `golden_success_cases`; the golden suite runs 75 + 20 (`ERR`) + 10
(`PARSE_ERR`) + 5 (`gen_trans_literals`) + 1 (usage) = 111 cases in 5 `#[test]`
functions. `NOTES.md:192-194` is otherwise accurate; make `README.md:8` match it.

### S2. `README.md:6` "~1.4k lines of Rust"
Measured: 1 646 non-comment implementation lines, 2 141 lines before the first
`#[cfg(test)]`, 3 300 lines in `src/` total (a `wc -l` reader sees 3.3k). Suggest
"~1.6k lines of implementation (~3.3k including the unit tests)".

### S3. `Ctx::names` is write-only, and `bind`/`define` are the same code twice
`Ctx::names` (`src/context.rs:13`) is written in `bind` (`:83-85`), `define`
(`:102-104`) and `subst_ctx` (`:137`) and never read — `ty_names()` (`:127`) reads
`types`. Delete the field or add a one-line "kept for parity with `Context.hs:11`"
comment. Related: `bind` (`:82-98`) and `define` (`:101-117`) differ only in the value
pushed into `env`; collapse to

```rust
fn push_binding(&self, x: Name, ty: Val, value: Val) -> Ctx {
    let mut names = Vec::with_capacity(self.names.len() + 1);
    names.push(x.clone());
    names.extend(self.names.iter().cloned());
    let mut types = Vec::with_capacity(self.types.len() + 1);
    types.push((x, ty));
    types.extend(self.types.iter().cloned());
    Ctx { lvl: self.lvl.inc(), names, types, env: self.env.push(value), pos: self.pos.clone() }
}
```

### S4. Dead public API (invisible to clippy because everything is `pub`)
`eval::subst_ne` (`src/eval.rs:58`) and `pretty::show_val_closed` (`src/pretty.rs:149`)
are referenced **nowhere**, not even in tests. `subst::empty_sub` (`src/subst.rs:39-40`)
is used only by its own test (`src/subst.rs:86-87`). `syntax::Raw::strip_pos`
(`src/syntax.rs:127`) is used only by the parser test helper (`src/parse.rs:448-452`).
`Env::is_empty` (`src/syntax.rs:216`), `impl Default for Env` (`src/syntax.rs:221-225`)
and `impl fmt::Display for Pn` (`src/syntax.rs:83-91`) are unused.
Delete them, or make them `pub(crate)` / `#[cfg(test)]`.

### S5. Unused parameters carried "for parity"
`eval::capp(_n: Lvl, ...)` (`src/eval.rs:210`; call sites `syntax.rs:249`,
`eval.rs:229, 350, 355, 469`) and `elab::unify_s(_d: Lvl, ...)` (`src/elab.rs:42`,
threaded at `:60`) take arguments that are never read. The comments already explain the
Haskell signature, so the explanation can stay while the parameter goes.

### S6. The core public API is undocumented
`cargo rustc --lib -- -W missing_docs` flags 15 public **functions** in `eval.rs`:
`subst_ne` `:58`, `subst_sp` `:65`, `subst_env` `:69`, `frc` `:85`, `frc_ne` `:127`,
`frc_ne_s` `:146`, `frc_sp` `:155`, `frc_sp_s` `:159`, `eval` `:264`, `conv_sp` `:326`,
`conv_cl` `:348`, `un_sub_ne` `:362`, `un_sub_ne_s` `:369`, `quote_val` `:378`,
`quote_ne` `:420` — plus `pretty::pretty_tm` (`src/pretty.rs:44`). `eval`, `frc` and
`quote_val` are the crate's real API and carry `impossible` preconditions
(`eval.rs:202-205, 316, 381, 422`). Document the preconditions or reduce visibility;
the rest of the crate is consistently documented, so this is an inconsistency.

### S7. `elab.rs:158-174` — guard + re-match + `unreachable!()`
```rust
Raw::Refl if matches!(ty, Val::Id(..)) => match &ty {
    Val::Id(_, x, y) => { ... }
    _ => unreachable!(),          // elab.rs:173
},
```
Rust 1.87 has `let ... else`:
```rust
Raw::Refl if matches!(ty, Val::Id(..)) => {
    let Val::Id(_, x, y) = &ty else { unreachable!() };
    ...
}
```
Better: extract the `infer`-then-compare fallback (`elab.rs:175-189`) into a helper so
`Raw::Refl` can use a plain `if let` and no `unreachable!()` remains.

### S8. `main.rs:58-61` throws away the panic payload
```rust
Err(_) => {
    eprintln!("error: internal panic while elaborating the program");
    ExitCode::from(101)
}
```
The port keeps invariant panics with precise messages (`eval.rs:202-205`,
`syntax.rs:381-387`, `elab.rs:279-283`); after this change a user sees only the generic
line. Downcast and print the payload:
```rust
Err(payload) => {
    let msg = payload.downcast_ref::<&str>().copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic");
    eprintln!("error: internal panic while elaborating the program: {msg}");
    ExitCode::from(101)
}
```

### S9. `elab.rs:370-384` `fmt()` is 15 lines for a 2-line job
The manual `find("{}")`/`push_str` loop, including its untested
`None => out.push_str("{}")` branch, is equivalent to:
```rust
fn fmt(template: &str, args: &[&str]) -> String {
    args.iter().fold(template.to_string(), |s, a| s.replacen("{}", a, 1))
}
```
Same left-to-right semantics, same behaviour for missing/extra arguments, 13 lines
fewer; `fmt_substitutes_placeholders_in_order` (`elab.rs:501-504`) keeps passing.

### S10. `lib.rs:67` doc comment is wrong
"Normalize a program and also return its (unnormalized) type" — `eval::quote_val` reads
a semantic value back to syntax, so the returned `Tm` *is* the type's normal form; it is
the same expression `run_named` prints in `Mode::Nf` (`src/lib.rs:80-84`). Say "and its
type (read back, i.e. normalized)".

### S11. `NOTES.md:92-104` (§3) claims behaviour the original cannot exhibit
§3 says the Haskell original "prints `λ x y . plus x y`, silently dropping the
argument". By §1 (`NOTES.md:76-78`) forcing a lambda diverges, and `quote` forces first
(`Eval.hs:216`), so the original cannot print *any* lambda — it diverges before the
quotation rule at `Eval.hs:231` is reached. Reword §3 as "the original's quotation rule
would drop the spine (once the divergence in §1 is fixed); the port keeps the intended
rendering". The same nuance applies to the "DEVIATION 1" comments in
`tests/golden.rs:38-42` and `:216-218`.

### S12. `tests/golden.rs:790-795` caret assertion is almost vacuous
```rust
if !lines[3].trim_end().ends_with('^') { ... }
```
Any caret column passes. The port reproduces `Main.hs:74-82` byte-for-byte
(`src/context.rs:28-47`), and I verified `let bad : Nat = U; U` emits exactly
`  |                 ^` (21 chars: `"  | "` + 16 spaces + `^`). Pin it:
```rust
let pad = " ".repeat(line.to_string().len());
let want_caret = format!("{pad} | {}^", " ".repeat(col - 1));
if lines[3] != want_caret { ... }
```
The stated reason for not pinning it (`tests/golden.rs:27-30`: the CLI contract and the
Haskell printer "can differ by a constant shift") is not true of this port.

### S13. `lib.rs:500-523` `deep_terms_run_on_a_large_stack` is weak
* `assert!(printed.starts_with("succ (succ"))` (`:522`) accepts *any* long `succ`
  tower; assert the exact expected normal form.
* `.join().expect("no stack overflow")` (`:520`) is misleading: a real stack overflow
  aborts the process (`0xC0000409`), it never returns `Err` from `join`; only a panic
  does.
* 400 levels is far below the observed failure point (~5 000), so the test documents
  the *thread stack size*, not the robustness limit, and it does not exercise
  `main.rs` at all. Pair it with the boundary case from M1.

### S14. DEVIATION 9 (`Val::Bot`) is undocumented and completely untested
The newest change adds `Val::Bot` (`src/syntax.rs:320-323`), `Tm::Bot => Val::Bot`
(`src/eval.rs:290-293`, "DEVIATION 9"), `(Val::Bot, Val::Bot) => true` in `conv`
(`src/eval.rs:321`) and `Val::Bot => Tm::Bot` in `quote_val` (`src/eval.rs:397`), and it
fixes a real crash: `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C` now
prints

```
λ A (!) . ⊥
  :
(A : U) → Id Nat (suc zero) zero → A
```

Two problems:
1. `src/eval.rs:290` and `src/syntax.rs:322` both point at "NOTES.md deviation 9", but
   `NOTES.md` still has only §1-§8 (`NOTES.md:68-168`) and its summary at `:64-66` says
   "Items 1–5 … items 6–8". Add §9 (with the crash it fixes) and update the summary.
2. No test anywhere evaluates an absurd-pattern lambda as a value — every `(!)` case in
   `src/lib.rs` (`:199, :213, :403`) and `tests/golden.rs` (`:115, :440, :472`) ends in
   `U`, so the new variant, the new `conv` arm and the new `quote_val` arm are dead as
   far as the suite is concerned. Add:
   * `eval::tests`: `eval(Env::empty(), &Tm::Bot) == Val::Bot`, `conv(0, &Val::Bot,
     &Val::Bot)`, `quote_val(Lvl::ZERO, &Val::Bot) == Tm::Bot`;
   * a golden case `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C` with
     the exact expected output above.

### S15. Test-only instrumentation in the hot path, read only by an `#[ignore]`d test
`src/eval.rs:19-20` declares `#[cfg(test)] pub static SUBST_SUB_CALLS: AtomicUsize`, and
`subst_sub` (`src/eval.rs:31-34`) increments it on every call:
```rust
pub fn subst_sub(s2: &Sub, s1: &Sub) -> Sub {
    #[cfg(test)]
    {
        SUBST_SUB_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    ...
```
Its only reader is `diag_trans_scaling` (`src/lib.rs:483-498`), which is
`#[ignore = "diagnostic"]`, so in a normal `cargo test` run the counter is written and
never read. It is `#[cfg(test)]`-gated (so it does not ship), but it is still noise at
the top of the port's most-called function and an undocumented global in `eval`. Either
delete both, or gate the counter behind a feature and mention it in `NOTES.md`. The
diagnostic itself is better placed in `benches/` or an example than in the unit-test
module where it shows up as "1 ignored".

---

## NICE TO HAVE

* **N1** `src/parse.rs:57-62`: `is_keyword` lists `"in"`, which the grammar never
  accepts (inherited from `Parse.hs:28`); it turns `in` into `unexpected keyword 'in'`.
  Comment it as inherited or drop it.
* **N2** `src/pretty.rs:44-46`: `pretty_tm` is a one-line wrapper over `go`; rename `go`
  to `pretty_tm` and delete the wrapper.
* **N3** `src/eval.rs:453`: `Pn::Abs => unreachable!("filtered out above")` is guarded
  ten lines earlier by `splittable` (`:441`). The message should name the invariant
  (e.g. `unreachable!("splittable excluded Pn::Abs")`), or the loop should skip
  `Pn::Abs` so no `unreachable!` is needed.
* **N4** `Error` (`src/context.rs:20-24`) and `ParseError` (`src/parse.rs:24-28`) are
  shape-identical and `src/lib.rs:48-51` hand-converts them; add
  `impl From<parse::ParseError> for Error` and use `map_err(Error::from)`.
* **N5** `src/lib.rs:341-348` re-derives the `gen_trans` string pieces inline; extract
  the five fragments into one helper used by both tests.
* **N6** Cost model: `Ctx::with_pos` (`src/context.rs:64-69`) clones `names`+`types`+
  `env` per `Raw::SrcPos`, `Ctx::bind`/`define` and `Env::push`/`extend`
  (`src/syntax.rs:199-210`) clone per binder, and `pretty::push` (`src/pretty.rs:37-42`)
  clones the name list per binder. Fine at this scale, but it is the port's main
  divergence from the Haskell `x:xs`/lazy-environment cost model — one sentence in
  `NOTES.md` "Rust-specific choices" would cover it.
* **N7** `src/main.rs:39-67`: the worker closure returns `(ExitCode, String, String)`;
  a small `struct Outcome { code, stdout, stderr }` would read better than positional
  tuple indices and remove the two `is_empty()` guards.
* **N8** `README.md:63-76`: the example is called "the README of the original project"
  but drops `cong`, which the parent README block contains (and which
  `tests/fixtures/readme_full.txt` keeps). Add `cong` or say "a subset of".
  (I ran the block exactly as printed — including the trailing `-- absurd pattern`
  comment — through the binary: exit 0, `U\n  :\nU`. The claim "All of these typecheck"
  at `README.md:78` is true.)

---

## What is genuinely good (do not churn it)

* **Fidelity of structure.** `syntax`/`subst`/`eval`/`elab`/`context`/`pretty`/`parse`
  map 1:1 onto the Haskell modules, and each has a module doc naming the file it ports.
* **Comments explain why, not what** — the deviation comments are the best part of the
  port (`eval.rs:22-29`, `:123-136`, `:213-220`; `elab.rs:302-306`;
  `syntax.rs:227-231`, `:341-344`).
* **Error handling is idiomatic and safe.** No `.unwrap()`/`.expect()` on user input
  outside tests; every panic is a documented invariant failure with a specific message;
  parse and elaboration errors carry `Pos` and render in the original's `displayError`
  shape (`src/context.rs:28-47`).
* **`Rc` vs `Box` is the right call and is documented.** `Rc` in every recursive
  `Val`/`Ne` field and in `Closure::body` mirrors Haskell's sharing (`syntax.rs:270-273`,
  `NOTES.md:50-55`); `Box` in `Raw`/`Tm` where no sharing is needed; `BTreeMap` for the
  level-indexed substitution is the closest std container to `IntMap`.
* **The golden suite is real black-box testing** with exact stdout/stderr/exit codes,
  error positions, source-line echo and caret shape, and it deliberately encodes the
  interface deviations. `readme_full_program` is the whole program from the parent
  README (verified against `../README.md:9-23`).
* **Unit tests assert behaviour.** `composing_after_solving_a_variable_narrows_the_domain`
  pins the `subst_sub` deviation numerically; `unify_threads_the_substitution_through_the_equations`
  covers the multi-equation path that fix enables; `a_stuck_refl_pattern_does_not_loop`
  and `forcing_is_idempotent_on_a_stuck_lambda` pin the anti-divergence fixes.
  I also verified the new mixed-pattern eta-expansion branch of `quote_ne`
  (`eval.rs:433-453`) by hand: `sym Nat zero zero` prints `λ refl . refl :
  Id Nat zero zero → Id Nat zero zero`, and `trans Nat zero zero zero` prints
  `λ refl refl . refl` at the expected type — worth adding as golden cases, since no
  current case partially applies a lambda whose remaining patterns include `refl`.

### Weakest tests / untested surface

* `conv_is_reflexive` (`src/eval.rs:623-627`) is the weakest test in the suite — it
  would pass with `conv = |_, _| true`; it is rescued by
  `conv_reduces_before_comparing` (`:630-634`) and `conv_distinguishes_different_values`
  (`:637-641`), so no action needed, but do not count it as coverage of `conv`.
* `plus_is_addition_on_numerals` (`src/lib.rs:239-250`) checks the port against
  `printed_nat` (`:137-144`), a re-implementation of the port's printer; the literal
  expectations in `normalization_of_closed_terms` (`:220-236`) contain the risk.
* Never exercised: `Val::Bot` (S14), `subst_ne`, `show_val_closed`, `empty_sub`,
  `Env::is_empty`, `Raw::strip_pos` (S4), `pretty_tm` only with `prec = 0`, and
  `main.rs`'s exit-2 paths (stdin read failure, thread spawn failure) plus the exit-101
  panic path. The last group is hard to test without fault injection; one sentence in
  `NOTES.md:183-196` would make the omission a decision rather than an oversight.
