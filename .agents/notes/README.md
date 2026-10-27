# Agent Notes

Durable design rationale for dpm-nbe: one decision per note, dated, written
when the reasoning would otherwise be lost (why an invariant exists, why an
obvious-looking change was rejected, why a deviation from the Haskell
original was chosen).

**Scope.** Write a note when a future contributor could plausibly re-litigate
or accidentally revert the decision — the frc strictness port is the
canonical example. Mechanical edits, local refactors, and bug fixes whose
reasoning fits in the owning code comment are exempt. Language-level
deviations from the Haskell original live in [../../NOTES.md](../../NOTES.md)
instead; a note may link one.

**Layout.** `implemented/<YYYY-MM-DD>-<slug>.md`. Notes are frozen once
written: if a decision is reversed, write a new note and link the old one
from it; never edit history. A note states: the decision, the alternative(s)
rejected, the evidence (probe/timing/test numbers), and what would have to be
true to revisit it.

**Index.** Keep this list current when adding a note:

- [2026-10-25 — frc stays whnf-only on rigid neutrals](implemented/2026-10-25-frc-strictness-port.md)
- [2026-10-26 — metas + pruning: a no-icit port of elaboration-zoo 05-pruning](implemented/2026-10-26-metas-pruning-port.md)
- [2026-10-28 — `let rec` fixpoints: reduction gated on constructors, termination checked structurally](implemented/2026-10-28-letrec-termination.md)
- [2026-10-28 — typeclasses: resolution is the ported unifier, sugar is the existing struct](implemented/2026-10-28-typeclass-resolution.md)
