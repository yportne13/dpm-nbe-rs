# Stress-area findings (randomized property tests, limits, scale)

Binary under test: `target/release/dpm-nbe.exe`, sha256 `7a919ad63518174b`
(pinned release build, black-box only).  Cases live in `cases/stress/` and are
(regenerable byte-identically) produced by `cases/stress/gen.mjs`
(`node gen.mjs`; seeds 1988 / 44444 / 20251025).

Oracle soundness: all 150 randomly generated well-typed arithmetic programs
(`rand-arith-*`) agree with the generator's value oracle, so **no soundness
findings** (no wrong normal form, no wrong type, no panic, no hang).

Below: two parked findings and the measured boundary/timing reference.

## 1. Numeral literals cannot exceed ~254 successors (parked, by design)

`suc (suc (... zero))` is a *parenthesised* nesting, and each level costs
about 2 units of the parser's shared `MAX_DEPTH = 512` budget.  Measured with
the prelude context used by the cases:

- `suc^254 zero` (depth-appnest/numlit family)  -> accepted
- `suc^255 zero`                                -> exit 1,
  `<stdin>:...: expression nesting is too deep`

Consequently the literal forms `suc^1000 zero` and `suc^10000 zero` from the
stress plan are **unrepresentable**: they are rejected with a clean exit-1
parse error long before evaluation.  Parked as
`scale-suc1000-literal.dpm.skip` and `scale-suc10000-literal.dpm.skip`
(sidecars record the expected exit 1 + `expression nesting is too deep`).

Repro (program on stdin of `dpm-nbe nf`; the full arithmetic prelude of
`rand-arith-*` cases followed by a literal with 255 `suc` wrappers around
`zero`; λ appears in the prelude, so feed it from a file rather than a shell
command line): exit 1, stderr `expression nesting is too deep`.

Judgment: not a bug — a deliberate stack-safety guard producing a proper user
error (exit 1, positioned diagnostic, no panic).  The practical consequence is
that large Peano numerals must be built with `plus`/`mult` (`scale-num-1000`
uses `plus` of four 250-literals, `scale-num-10000` uses `mult 250 40`), which
the evaluator handles fine.

## 2. Normalisation of large numerals is super-linear (timing finding)

Measured one-off wall times of the pinned binary (Windows, same host as the
suite run):

| program                                | NF successors | time     |
|----------------------------------------|--------------:|----------|
| `plus` of four 250-numerals            |  1 000        | 0.66 s   |
| `plus 1000 1000` (via a `let`)         |  2 000        | 2.2 s    |
| `mult 25 25`                           |    625        | 0.16 s   |
| `mult 250 40`                          | 10 000        | 43.2 s   |
| elim-based `fib 18`                    |  2 584        | 6.1 s    |
| `power 2 12`                           |  4 096        | 9.7 s    |

1000 -> 10 000 successors is a 10x size increase but a ~66x time increase
(~O(n^2)-ish in the successor count), so producing a ~60 KB numeral normal
form (`scale-num-10000`) costs ~43 s.  Judgment: performance observation, not
a correctness bug — every run exits 0 with the exactly-correct normal form,
nothing hangs or panics; `.timeout` sidecars carry ~3x headroom (130 s for
`scale-num-10000`, 20 s for `fib 18`, 30 s for `power 2 12`).

## 3. Measured depth/width boundaries (reference)

All boundaries measured against the pinned binary with the exact source
context the committed cases use (the arithmetic prelude itself consumes a few
depth units; "fail" is always exit 1 with `expression nesting is too deep`):

| nestable form                       | last pass | first fail |
|-------------------------------------|----------:|-----------:|
| application nesting `f (f (...))`   |       253 |        254 |
| lambda nesting (under `let`)        |       506 |        507 |
| `let` chains                        |       507 |        508 |
| parenthesised atoms `(((zero)))`    |       253 |        254 |
| numeral literal `suc^N zero`        |       254 |        255 |
| projection chains `x.b.b...b`       |       512 |        513 |
| data parameter groups               |       512 |        513 |
| application width (arguments)       |       511 |        512 |
| constructor width (arguments)       |       511 |        512 |
| struct field count                  |       511 |        512 |

The differences are legitimate consequences of how the grammar is counted:
parenthesised nesting (applications, parens, numeral literals) spends ~2
budget units per level against the shared `MAX_DEPTH = 512`; binder forms
(`λ` bodies, `let` bodies) spend ~1 unit per level and start ~5 units deep in
the prelude context; postfix projections, data parameter groups and spine
atoms have their own `> 512` counters, which is why those forms really do
break at 512/513 or 511/512.  A parameterised datatype with 512 parameters
must be declared with zero constructors, because the mandatory ctor result
`D p1 ... p512` would itself need a 513-atom spine.

The committed `depth-*` cases pin 511/512/513 for every form plus the exact
boundary pair above; the `width-*` cases pin 511/512/513 arguments, 500
chained lets, and 200/511/512 struct fields; `det-*` are byte-identical pairs
(arithmetic, `fib 18`, deep lambda nest) that must produce identical output.
