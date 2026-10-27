# 2026-10-28 — Typeclasses: resolution is the ported unifier, sugar is the existing struct

## Decision

The `typeclass` branch adds typeclasses (NOTES.md deviation 20) with
maximal reuse of what the last two branches built:

1. **A class is a struct** (deviation 16 sugar, untouched): the
   single-constructor datatype, `mkC`, and per-method projections all come
   from the existing machinery.  The only addition is a registry entry, and
   the *whole feature* lives in how the projections' class-typed `(q : C
   ps)` domains are consumed.
2. **An instance is a `let` plus a table entry**: the checked value and
   type close like stored constructor types (no enclosing binders), and a
   thread-local table is scanned newest-first — later declarations shadow
   earlier ones, the standard local-covers-global approximation.
3. **Resolution is the deviation-18 unifier**: fresh metas for an entry's
   telescope, `unify` the codomain against the target, ordinary domains
   must come out solved (they are the arguments), class domains resolve
   recursively.  Failed entries roll back via `meta::snapshot`/`restore` —
   a new `meta` API that copies the entry vector; solutions are `Rc`
   values, so snapshots are cheap, and id allocation is restored
   implicitly because ids derive from the vector length.
4. **Insertion is at the application site only**: elaborating an
   application skips the function type's leading class-application
   domains, filling each with a resolved dictionary.  The escape hatches
   are the field projection `d.show` (a value projection, never an
   application) and the explicit `[T]` expression.  No implicit-argument
   machinery, no binder annotations — the language stays explicit-only.

The search is depth-limited (`MAX_DEPTH = 64`), which terminates cyclic
instance sets by failure (`instance : Eq A -> Eq A` searches to the bound
and reports *no instance*) at the cost of rejecting legitimately deep
chains beyond 64 — a trade documented rather than solved (a visited-set
would need value-level equality on targets, which is undecidable here).

## Alternatives rejected

- **Lean-style instance-implicit binders with elaboration-time insertion
  everywhere**: requires the implicit-argument machinery the metas branch
  deliberately did not port, for one use case.  Application-site insertion
  gives the same ergonomics for the dominant use (bare method references)
  without touching the Pi discipline.
- **Coherence by uniqueness check at declaration time** (rejecting
  overlapping instances): would forbid the shadowing that makes local
  instances useful, and needs decidable target equality.  The port
  documents incoherence as a restriction instead.
- **Backtracking via error-collecting unification** (making `unify`
  return attempted-solutions for rollback): invasive.  Snapshotting the
  metacontext is three lines and keeps `unify`'s failure-is-exception
  contract, which the `check_pm` interplay (deviation 18) depends on.

## Evidence

- Probe matrix, all real runs: `show Nat x` auto-inserts; `[Show Nat]`
  prints the found dictionary; `showList`'s constraint resolves
  recursively through `d.show` in its own body; a later instance shadows
  an earlier one; a failed-subgoal entry is skipped for one that matches;
  `no instance of Show Bool`, `must end in a class application`, `may not
  depend on an enclosing binder`, and duplicate-method errors all fire.
- Two implementation post mortems worth keeping: (a) the class originally
  registered *after* its body — the body is the rest of the program, so
  the class's own instances failed resolution; fixed by `elab_data`'s
  `class` flag registering before the body.  (b) `try_entry`'s constraint
  walk re-applied already-applied arguments (returning `None` on a
  non-Pi), and unsolved constraint metas leaked to the end-of-program
  check — both caught by the probe matrix, both fixed in `tc.rs`.
- 177 workspace unit tests (6 new typeclass families + parse tests), 4
  golden rows, 6 blackbox cases; scoped suites green at commit time.

## What would have to be true to revisit

- Implicit arguments would reopen the insertion design entirely (binder
  icits instead of domain-shape insertion).
- Coherence would need either unique-instance declarations or a
  canonicalization order on the table; both conflict with shadowing.
- Superclasses/deriving are orthogonal sugar on the same table; the
  registry and resolution would not change.
