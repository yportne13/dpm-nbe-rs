# AGENTS.md

dpm-nbe is a Rust port of the Haskell dependent-type elaborator `dpm-nbe`
(sibling checkout at `../../Haskell/dpm-nbe`): bidirectional elaboration,
NbE evaluation, user-declared inductive types with generated eliminators,
`struct` sugar with `x.f` projections, and a predicative universe tower with
cumulativity. Read [NOTES.md](NOTES.md) — every deviation from the Haskell
original is numbered there and was probe-verified against the binary; treat it
as the language specification deltas, not as history.

## Repository layout

```
src/         the elaborator library + CLI (elaborate: context, elab, eval,
             parse, pretty, subst, syntax, testdata; entry: lib, main)
lsp/         language server crate (analysis, tokens, hover)
web/         browser IDE: wasm API under web/api, static app under web/www,
             build + smoke tooling under web/tools
vscode/      editor extension (syntaxes/, client/)
tests/       golden.rs: byte-pinned CLI expectations (nf/type output,
             elaboration and parse errors)
blackbox/    black-box CLI suite: run.sh runner, fuzz.mjs fuzzer,
             cases/{lang,errors,stress}, findings-*.md (see
             blackbox/README.md)
reviews/     frozen review reports from the port milestone (semantics vs
             the Haskell original, robustness, quality) — reference only
scripts/     repo helper scripts
.agents/     skills (task playbooks) and notes (decision rationale)
```

## Commands

```sh
cargo test --workspace          # unit + golden (root) and lsp tests
cargo build --release           # binary for everything below
blackbox/run.sh                 # full black-box suite (~2 min, 410 cases)
blackbox/run.sh blackbox/cases/lang   # one area only
node blackbox/fuzz.mjs          # randomized no-panic fuzz, deterministic seed
node web/build.mjs              # build the wasm language server + web assets
node web/tools/smoke.mjs && node web/tools/client-smoke.mjs
npm run bundle --prefix vscode/client
node vscode/client/test/transport.test.mjs && node vscode/client/test/jsonrpc.test.mjs
```

The CLI under test: `dpm-nbe nf|type` reads a program from stdin; `nf` prints
`<normal form>\n  :\n<type>`, `type` prints the type; user errors go to
stderr with a `<stdin>:L:C:` position. Exit codes: 0 success, 1 user error,
2 CLI/IO misuse, 101 internal panic — 101 is always a bug.

**Windows/λ caveat:** a literal `λ` breaks Git-Bash command lines; write
`\ x . t` on the command line and keep `λ` inside files only.

## Conventions

- **Probe, then claim.** Every behavioral statement in docs, reviews, or
  discussions is verified against the built binary first
  ([dpm-probe](.agents/skills/dpm-probe/SKILL.md)). NOTES.md deviations in
  particular must cite probe-verified behavior, never source reading alone.
- **Stored constructor types are closed**: after `subst_outer`, a stored ctor
  type's only free variable is the datatype itself; parameters and fields
  follow the `[D, params, fields]` context convention that `elim_ty`,
  `infer_proj`, and `close_data` all rely on. Any new consumer must keep that
  invariant or extend it deliberately.
- **`frc` is whnf-only on rigid neutrals** (`Ne::Var/Data/Ctor` arms return
  without forcing spines). This is a deliberate laziness-port deviation and a
  100–460× performance cliff — do not reintroduce eager spine forcing; the
  quadratic history is recorded in
  [notes](.agents/notes/implemented/2026-10-25-frc-strictness-port.md).
- **Conversion is beta-only** (no eta), even though quotation eta-expands
  lambdas when printing. Cumulativity exists only in the check direction
  (`conv_le`: `U i ≤ U j` for `i <= j`, Pi domains invariant, codomain
  covariant); `conv`, `refl`, and unification stay strict.
- **Eliminators give no induction hypothesis for function-typed arguments**
  that merely end in the datatype (`Nat -> D` is legal, non-recursive).
  Constructors may not store arguments from a higher universe (deviation 17);
  `struct` fields may not depend on earlier fields.
- **Parser depth budget** (`MAX_DEPTH = 512`) is shared unevenly: paren atoms
  cost 2 units/level (boundary 253/254), lambdas 506/507, lets 507/508,
  projections and param groups 512/513, spine width 511/512. Measured values
  live in blackbox/findings-stress.md; changing the parser means re-pinning
  those cases.
- **Error messages are contract.** golden.rs and blackbox pin exact fragments
  and positions; rewording one is a breaking change to those tables and needs
  the owning rows updated in the same change.
- **Pure-performance changes must keep output byte-identical**; the blackbox
  `.out` sidecars are the oracle ([dpm-perf-check](.agents/skills/dpm-perf-check/SKILL.md)).
- Style: match the local idiom, including the double-match arm pattern in
  `infer` (`Raw::Data(..)` guard + inner destructure). Comments state
  constraints, not narration; no comment restates what code says. Doc
  comments on `conv`/`classify_arg`-style functions carry the spec — keep
  them truthful or fix them with the code.
- **Notes over comments for decisions**: durable design rationale goes to
  [.agents/notes/](.agents/notes/README.md) (dated, one decision per note);
  NOTES.md owns language-level deviations; code comments own local
  constraints only.

## Testing policy

Match evidence to the surface: elaborator semantics → unit tests in
`src/lib.rs` next to the feature plus golden rows; CLI contract (output
bytes, exit codes, positions, encoding) → blackbox cases; robustness against
malformed input → `fuzz.mjs` seeds; web/vscode → their own smoke/bundle
tests. Never run the full blackbox suite to iterate on a single case — scope
it by directory. Before pushing, follow
[dpm-pre-push-checks](.agents/skills/dpm-pre-push-checks/SKILL.md).

Bugs found while testing are parked as `NAME.dpm.skip` with an entry in the
area's `findings-*.md`, never silently dropped and never expected to fail.

## Editing these instructions

Keep each rule self-contained, probe-verified, and linked to its executable
source of truth (NOTES.md deviation, blackbox case, or note). Condense when
clarity survives; do not add rules that name no concrete artifact.
