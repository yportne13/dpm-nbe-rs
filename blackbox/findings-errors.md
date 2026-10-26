# Findings — error paths, diagnostics, positions, encoding (`cases/errors/`)

Binary under test: `target/release/dpm-nbe.exe`, sha256 `7a919ad63518174b…`
(the pinned release binary; never rebuilt while writing these cases).
Method: probe the binary by hand, then pin **actual** stderr fragments, exit
codes and (for success contrasts) stdout.  Every case was probed before it was
written; nothing is pinned on faith.

Suite state: **87 cases (incl. the seed), 87 passed, 0 failed, 0 skipped.**
No case exits 101, prints an internal panic, or hangs.

## Inventory

| group | cases | flavor |
|---|---|---|
| parse-* | 22 | stray char `@`/`!`/`#`, `expected an atom` after `=`, keywords misused as identifiers (`let let` → `expected '_'`, `p.where` → `'where' is a keyword`, `suc let` → position pinned), dangling `p.`, unbalanced `(`, missing let body, stray `)`, missing `:`/`;`/identifier in `data`, `Id` arity, `\ (!)` as first pattern, `U1073741824` / `U99999999999999` (`universe level is too large`), unterminated `{-`, `λ . refl` (no patterns) |
| elab-* | 31 | unbound (incl. inside a let, `type` mode), `Expected a function type`, type mismatch (expected/inferred), `Can't infer type for` lambda/refl, `expecting zero equals to suc zero, but it is not`, refl-should-be-absurd / absurd-should-be-refl / not-sure hints, `Too many patterns.`, refl/absurd pattern on non-`Id`, `elim` on `U`/on an instance/on a function/bare, projection on multi-ctor data / on a Pi / on a universe / missing field (fields listed), duplicate struct field, field depending on an earlier field, ctor-must-return-datatype, double-negation strict positivity, data-not-in-a-universe (`Nat` and Pi type), ctor universe guard, `succ` not reserved |
| ok-* | 6 | `U1073741823` type-checks; partial `elim Nat` prints its recursor type; `s.a`; absurd-lambda value prints `λ A (!) . ⊥`; `Wrap1 : U1` storing `U`; recursive struct field |
| pos-* | 8 | error on line 3 / line 4 / line 10, deep inside nesting, after a >300-char first line, tab-indented line (col 18 = tab counts one column), multi-line block comment before the error, `U → x` (multi-byte char = one column), CRLF line/column parity |
| enc-* / longline-* | 11 | BOM (exit 0), BOM+CRLF (exit 0), invalid UTF-8 (exit 1 `input is not valid UTF-8`), empty input, whitespace-only, CRLF-valid (exit 0), NUL byte, 100 000-char identifier, two long identifiers, late error on a 401-column line |
| depth-* | 9 | see boundaries below |

`blackbox/cases/errors/gen.sh` (re)generates the byte-level and oversized
cases; run `bash blackbox/cases/errors/gen.sh` once after checkout.  All other
cases are literal files.  The generator is idempotent.

## Pinned from `reviews/robustness.md` (F1–F5)

* **F1 keyword error position** — `parse-keyword-app-position` (`suc let` →
  `<stdin>:2:5:`, caret on the keyword start), plus `let let` and `p.where`.
* **F2 BOM acceptance** — `enc-bom-success`, `enc-bom-crlf`: exit 0 with the
  correct normal form (NOTES.md deviation 11).
* **F3 line-window truncation** — `longline-late-parse-error` pins the
  windowed echo (`1 | …`, error at 1:401); `pos-long-first-line` pins that a
  long earlier line does not corrupt the next line's position.
* **F3r name truncation** — `enc-ident-100k` pins `unbound variable: ` + 64
  chars + `…`; measured stderr = **277 bytes** for a 100 000-char identifier.
  `longline-late-unbound` pins the same for a mid-line name.
* **F4 tab = one column (by design)** — `pos-tab-indent` pins `<stdin>:3:18:`
  for a tab-indented `let bad : Nat = U;`.
* **F5 MAX_DEPTH = 512 (shared budget)** — measured boundaries on this binary,
  each pinned on both sides (ok case exit 0 / reject case exit 1 with
  `expression nesting is too deep`):

  | form | accepted | rejected |
  |---|---|---|
  | nested parentheses | 255 (`depth-parens-255-ok`) | 256 (`depth-parens-256`) |
  | chained top-level `let`s | 510 | 511 |
  | lambdas inside one `let` (type+value share) | 509 | 510 |
  | bare nested lambdas | 510 parses (then `Can't infer type for lambda expression`) | 511 |
  | flat application spine | 512 atoms parse (ordinary unbound error) | 513 atoms |

  Bounded stderr for all rejects: 59–377 bytes; every depth rejection returns
  in well under the 10 s case timeout.

## Judgment calls

* **CRLF — accepted, and I judge that correct.**  `enc-crlf-valid` (exit 0,
  `suc zero : Nat`), `enc-bom-crlf` (exit 0) and `enc-crlf-error-pos`
  (`<stdin>:2:5:`, identical line/column to the LF twin) pin that CR is
  whitespace.  This contradicts nothing: `blackbox/README.md` is silent on
  line endings, and NOTES.md documents BOM-as-whitespace (deviation 11) but
  makes no LF-only promise; the old review listed CRLF under "clean".  A
  parser that *rejected* CRLF-only files would break every Windows-edited
  program for no reason.  Pinned as a permanent expectation.
* **Empty input → exit 1** (`expected an atom` at `<stdin>:1:1:`), i.e. a
  parse error, not CLI misuse (2).  Matches the golden suite; judged right —
  the CLI contract reserves 2 for argument/IO trouble, and stdin was read fine.
* **Whitespace-only input** behaves the same, with the position at the end of
  input (`<stdin>:3:1:`).  Judged fine.
* **NUL byte** after a complete program is trailing garbage: exit 1,
  `expected end of input` at 1:2.  NUL is valid UTF-8, so the UTF-8 gate
  correctly does not fire; the lexer treats it as an atom-ish unknown.  Bounded
  and clean; pinned in `enc-nul-byte`.
* **There is no "unexpected character" diagnostic** on this binary.  Stray
  characters surface context-sensitively as `expected an atom` (`@`, `!`),
  `expected end of input` (`$`, stray `)`, NUL) or `expected ':'` (`#` in a
  binder).  The cases pin the actual messages.
* **`succ` is not a reserved word here — NOTES.md deviation 12 is stale.**
  NOTES.md says the port accepts `succ` as a synonym and that the printer
  writes `succ`; on this binary `succ zero` is `unbound variable: succ`
  (pinned in `elab-succ-not-reserved`) and numerals print as `suc`.  The
  behavior is self-consistent (the printer's output is valid input) and
  matches `tests/golden.rs`, so this is documentation drift, not a bug.
  If the synonym is ever (re)introduced, that case will flag it.
* **Field depending on an earlier field** (`struct S : U where a : Nat;
  b : Id Nat a a;`) surfaces as `unbound variable: a` at the field's use site
  — the user never sees the generated-projection check NOTES.md describes.
  Pinned actual; message is accurate enough (the field is genuinely not in
  scope) and points at the right column.
* **Depth-error position at EOF (cosmetic).**  `depth-spine-513` reports
  `<stdin>:2:1:` with an empty echoed line — the budget is exhausted at end of
  input rather than at the atom that tipped it.  Bounded and unambiguous;
  accepted as-is.  (Without a trailing newline the same input reports the
  mid-line column, so the position depends on where the parser stands when the
  budget runs out.)

## Parked cases (`.dpm.skip`)

None.  Nothing produced exit 101, an internal panic, or a hang: the
robustness review's clean bill held up under 87 fresh probes.
