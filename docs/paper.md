# Pruning, Twice: Declaration Reachability and Metavariable Dependencies in dpm-nbe-rs

**A formal analysis of the `prune` and `pruning` branches.**
Companion Lean formalization: [`lean/Prune.lean`](lean/Prune.lean),
[`lean/PatternUnif.lean`](lean/PatternUnif.lean) (theorem names cross-referenced
as `Lean ⟨name⟩`).

## Abstract

This note analyses two features that both answer the question *"what can we
delete without changing what the program means?"* — under two entirely
different notions of "dependency". The `prune` branch adds a CLI mode that
removes *top-level declarations* the goal cannot reach, a purely syntactic
reachability problem over a linear binding structure; we prove that pruning
preserves name resolution and hence elaboration. The `pruning` branch ports
the elaboration zoo's `05-pruning`: metavariables solved by pattern
unification, where *pruning* deletes a metavariable's dependency on spine
arguments its solution never uses; we prove the two soundness statements the
algorithm rests on — that renaming under an inverted spine inverts
substitution, and that a pruned solution still solves the equation whenever
the pruned *type* is well-formed. Both are machine-checked at the scope
level in Lean 4; the definitional-equality layer is left as an oracle.

---

## 1. Introduction

dpm-nbe-rs is a Rust port of a bidirectional elaborator for a dependent type
theory with user-declared inductive types, a predicative universe tower with
cumulativity, first-class identity types with `refl`/absurd pattern matching,
and NbE-based evaluation. Two branches extend it:

* **Branch A — `prune`.** A third CLI mode, `dpm-nbe prune`, that drops the
  top-level declarations the final expression cannot name and prints the
  program back. The notion of dependency is *lexical*: a declaration depends
  on another iff the other's name occurs free in it. The theorem is that
  lexical deletion commutes with scoping (§3).

* **Branch B — `pruning`.** Holes `_`, a typed metacontext, and pattern
  unification with pruning, non-linear spine solving and intersection, after
  the elaboration zoo's `05-pruning`. The notion of dependency is
  *semantics-shaping*: a meta's *arguments* may be deleted from its type
  when no solution can observe them. The theorems are the inversion lemma
  for partial renamings and the well-typedness side condition of pruning
  (§4).

The two share more than the name: both are reachability analyses whose
soundness reduces to a *scoping invariant* — A's "the innermost provider of
a name is kept whenever a use is" (Lemma 3.6), B's "the partial renaming
numbers the use-site binder exactly as the meta-context grows" (Invariant
4.5). Both invariants are what the Lean files verify.

## 2. The base theory 𝒯

We summarise only what the two branches interact with. Full rules are in
[NOTES.md](../NOTES.md) and the Haskell lineage.

**Syntax.** Surface programs are declaration chains: a program is
`d₁ ; d₂ ; … ; dₙ ; e` where each `dᵢ` scopes over the rest and `e` is the
goal. Declarations are `let x : A := t`, `data D (p⃗) : Uᵗ where cⱼ : Bⱼ`, and
`struct S (p⃗) : Uᵗ where fⱼ : Bⱼ` (sugar for a single-constructor `data`
with constructor `mkS` plus one projection `fⱼ` per field). Expressions are
those of a Π-type theory with a universe tower `U₀ : U₁ : U₂ : …` (written
`U` for `U₀`), identity types `Id A u v`, `refl`, λ with variable, `refl`
and absurd `(!)` patterns, `elim D`, and `x.f` projections.

**Typing.** Bidirectional (`check`/`infer`), with the check direction using
cumulative subtyping `conv_le`: `Uᵢ ≤ Uⱼ` iff `i ≤ j`, Π-domains invariant,
Π-codomains covariant; definitional equality (`conv`, `refl`, unification)
stays strict and β-only — no η at the conversion level. Data declarations
generate eliminators `elim D : Π p⃗ (P : D p⃗ → U) → methods → (x : D p⃗) → P x`
with induction hypotheses only for arguments whose type *is* `D p⃗`
(functions *into* `D` are non-recursive). Pattern `refl` on `Id A t u`
unifies `t ≐ u` by an index unifier returning a context substitution,
an absurdity, or *not sure*; a `with-K` system throughout.

**Evaluation.** NbE with closure values; forcing (`frc`) is whnf-only on
rigid neutrals (variables, data formers, constructors) — spines of rigid
heads are forced on demand, never eagerly.

## 3. Branch A: declaration pruning is resolution-preserving

### 3.1 Programs as chains

**Definition 3.1 (program).** A program is a pair `P = (d⃗, e)` of a finite
sequence of declarations and a goal expression. Write `d⃗⟨i⟩` for the `i`-th
declaration (0-based, outermost first) and `|d⃗|` for its length.

Each declaration decomposes into the names it *binds* and the expressions it
*uses*:

**Definition 3.2 (binds/uses).**

```
binds (let x : A := t)      = {x}                  ( {} if x = "_" )
binds (data D p⃗ … where cⱼ) = {D} ∪ {cⱼ}
binds (struct S p⃗ … where fⱼ) = {S, mkS} ∪ {fⱼ}    -- the sugar's generated names

uses  (let x : A := t)      = fv(A) ∪ fv(t)
uses  (data/struct …)       = fv(p⃗-types) ∪ fv(Uᵗ) ∪ fv(field/ctor types)
```

where `fv` collects free *surface* names, tracking local binders (λ/Pi/let
and nested declarations bind). Parameters and the datatype itself are
excluded from `uses` (they are binders *within* the declaration); a
projection's field label `x.f` contributes `fv(x)` but not `f` — `f` is a
label of `x`'s type, not a name in scope. The `struct` case is the one place
`binds` must mirror the elaborator rather than the surface syntax: the
desugaring (NOTES.md §16) puts `mkS` and each `fⱼ` into scope as ordinary
`let`s, so they are bound names.

**Definition 3.3 (resolution).** For a use of `x` at *use site* `j` — the
goal (`j = |d⃗|`) or a use inside `dⱼ` — the *provider* is

```
ρ(d⃗, j, x) = max { i < j | x ∈ binds(d⟨i⟩) }        (None if empty)
```

i.e. the innermost binding declaration. This is exactly the order
`Ctx::lookup` elaborates with (newest binding first).

**Lemma 3.4 (provider uniqueness).** `ρ` is a partial function of `(d⃗, j, x)`;
when defined it is the unique `i` with `x ∈ binds d⟨i⟩` and no `k` with
`i < k < j` and `x ∈ binds d⟨k⟩`. ∎

### 3.2 Reachability

**Definition 3.5 (reachability).** `Reach ⊆ {0..|d⃗|}` is the least set with

```
(R-goal)  ρ(d⃗, |d⃗|, x) = i  (x ∈ fv(e))              ⟹  i ∈ Reach
(R-decl)  x ∈ uses(d⟨j⟩) ∧ ρ(d⃗, j, x) = i             ⟹  i ∈ Reach
(R-step)  j ∈ Reach ∧ x ∈ uses(d⟨j⟩) ∧ ρ(d⃗, j, x) = i ⟹  i ∈ Reach
```

The implementation computes `Reach` as a worklist closure — confluence of
the closure operator makes the two definitions equal; determinism of `ρ`
(Lemma 3.4) is what makes the worklist's choice of exploration order
irrelevant.

**Definition 3.6 (pruned program).** `P ↾ Reach = (d⃗ kept in order, e)`.

### 3.3 Soundness

**Lemma 3.7 (kept providers).** If `ρ(d⃗, j, x) = i` for a use site `j` of a
declaration that is kept (`j ∈ Reach`, or `j = |d⃗|`), then `i ∈ Reach`.

*Proof.* Immediate by (R-decl)/(R-step): the rule that fires for the pair
`(j, x)` puts exactly `i = ρ(d⃗, j, x)` into `Reach`. ∎

**Theorem 3.8 (no rebinding).** Let `K = Reach`, `P' = P ↾ K`, and
`π : {i | i ∈ K} → {0..|d⃗'|}` the order-preserving reindexing. Then for every
use site `j` of a kept declaration (or the goal) and every name `x`:

```
ρ(d⃗', π j, x) = π (ρ(d⃗, j, x))       whenever ρ(d⃗, j, x) is defined
```

*Proof.* Let `i = ρ(d⃗, j, x)`. By Lemma 3.7 `i ∈ K`, so `π i` is defined and
`x ∈ binds(d'⟨π i⟩)`. Suppose some `k` with `π i < k < π j` had
`x ∈ binds(d'⟨k⟩)`; then `k = π m` for an `m` with `i < m < j` and
`x ∈ binds(d⟨m⟩)` (π is order-preserving and surjective onto kept indices),
contradicting the maximality of `i` (Lemma 3.4). Hence `π i` is the innermost
surviving provider. ∎

**Theorem 3.9 (elaboration invariance).** For every program `P` that
elaborates, `P` and `P ↾ Reach` elaborate to the same core term and type.

*Proof.* The elaborator consumes a declaration chain by folding `check`/`infer`
through a context extended one declaration at a time. Two contexts related
pointwise by equal name→value/type maps drive the elaborator identically
(*locality*: declarations influence later elaboration only through the names
they bind — the theory has no instances or implicit effects). By Theorem 3.8
the name→declaration map at every use site is the same in `P'` as in `P`; the
bound values are untouched by pruning (whole declarations are removed, never
rewritten). Induction over the kept chain. ∎

Declarations that fail to elaborate are invisible to the theorem's
hypothesis, and pruning does not typecheck: a program `P` *need not* elaborate
for `P ↾ Reach` to be formed, and if the unreachable declarations are the only
ill-typed part, `P ↾ Reach` elaborates while `P` does not. That is the
feature, not an omission.

**Proposition 3.10 (complexity).** `Reach` is computed in
`O(Σᵢ |uses(dᵢ)| · α)` name resolutions, each a scan bounded by `|d⃗|`;
`α` is the (tiny) transitive fan-in. Output printing is linear in the kept
program. ∎

## 4. Branch B: metas, pattern unification, pruning

### 4.1 Metavariables and their types

**Definition 4.1 (meta).** A meta is a pair `(m, τₘ)` where `τₘ` is a
*closed* iterated-Pi type `Π (x₁:A₁) … (x_k:A_k). B`. A term `?m t⃗` is
well-formed in Γ when `|t⃗| ≤ k` and the applications type-check. The
metacontext Δ maps `m` to `Unsolved τₘ` or `Solved (v, τₘ)` with `v` a
closed value of type `τₘ`.

Fresh metas are created in a context Γ with type a *value* `A`:

```
fresh : Γ ⊢ A type  ⟹  Δ, ?m : Π(bound Γ). A[bound Γ]  ⊢  ?m boundΓ  ∶  A
```

where `bound Γ` are the variables of Γ (entries whose environment value is
their own variable), in order. The zoo writes the term as
`AppPruning (Meta m) (pruning Γ)`; the port writes the application chain out.
Defines contribute neither a Π nor an argument — the *mask* of Γ, in the
terminology below.

At the value layer a meta head is **flexible**: forcing `?m t⃗` with `m`
solved unfolds the solution applied to `t⃗`; unsolved it is its own whnf.
This is compatible with the whnf-only-on-rigid discipline of `frc` because a
meta is not rigid — there is nothing to preserve by not unfolding it.

### 4.2 Partial renamings

A solution of `Γ ⊢ ?m sp ≐ rhs` (spine `sp` in Γ, length = the meta's
arity `k`) is a term of the *meta's* context. The bridge is a partial
renaming:

**Definition 4.2 (PRen).** A partial renaming from Γ (size `γ`) to Δ (size
`δ`) is a map `σ : Lvl_γ → Lvl_δ` (partial). With `lift`/`skip`:

```
lift σ = σ ∪ { γ ↦ δ }   (both sides grow)      -- the use-site binder's
skip σ = σ                (γ side grows)        -- level is the old γ, which
                                                   is what opening a closure
                                                   at level γ produces
```

**Invariant 4.3.** `rename` outputs a term of the Δ-side; lifting numbers the
use-site binder with the old `γ`, and simultaneously extends Δ by one — so
`Var (lvl2ix δ' newΔ)` lands exactly on the binder the output adds. (The zoo's
comments state this the other way round; the port pins it in
`unify.rs`'s header and the Lean file proves the index discipline.)

**Definition 4.4 (invert).** For a spine `sp` of Γ-values, `invert(sp)`
succeeds when every entry forces to a variable, returning `σ = { sp⟨i⟩ ↦ i }`
together with, when some variable occurs twice, the *non-linearity mask*
`ν : k → Bool` (`ν i = ⊤` iff `sp⟨i⟩` is a repeated variable).

**Lemma 4.5 (invert on linear spines).** If `sp` is pointwise distinct
variables, `σ = invert(sp)` is injective and defined exactly on
`{sp⟨i⟩}`. ∎

### 4.3 The renaming and the inversion theorem

`rename` is the judgement (meta-variables included; `occ` carries the meta
being solved):

```
occ = m                                  (occurs check fails)
──────────────────────────────  (Rn-flex)
rename σ (?m t⃗) = prune-or-rename …     (Fig. 4 when pruning applies)

σ(x) = x'                                (Rn-var)
──────────────────────────────
rename σ x = Var (lvl2ix δ x')

            rename (lift σ) (t[x↦γ]) = u
────────────────────────────────────────  (Rn-lam)
rename σ (λx. t) = λx. u        (and likewise Π, spine congruences)
```

The theorem the whole algorithm rests on:

**Theorem 4.6 (inversion).** Let `sp : Fin k → Lvl_γ` be injective with
`σ = invert(sp)`, and suppose `rename σ rhs ⇓ rhs'` (some `rhs'` of the
meta-context). Then substituting the spine back recovers the right-hand side:

```
rhs'[xᵢ ↦ sp⟨i⟩]  =  rhs.
```

*Proof.* Induction on `rhs`.

* `x = sp⟨i⟩`: `σ x = i` so `rhs' = Var i`, and `Var i [xᵢ ↦ sp⟨i⟩] = Var (sp i) = rhs`.
  If `x` is not in the image of `sp`, `rename σ x` is undefined — excluded.
* `λ t`: the binder is numbered `γ` and `lift σ` maps it to `δ`; the lifted
  spine `sp⁺` (with `γ+1 ↦ δ+1` for the new binder) satisfies the hypotheses,
  so the IH applies under the binder. The environment of the substitution on
  the left extends by `Var γ` on the nose.
* Applications, Π, and meta spines are congruences. ∎

`Lean rename_inversion` — proved for the variable/meta/application/λ fragment
in [`lean/PatternUnif.lean`](lean/PatternUnif.lean).

### 4.4 Solving

**Definition 4.7 (solution).** `solve(γ, m, sp, rhs) = invert sp`, then
`rename` (with `occ = m`), then

```
Δ(m) := λᵗ⃗. rhs'          (λᵗ⃗ takes binder names from τₘ's Π-chain)
```

**Theorem 4.8 (solution soundness).** If `solve(γ, m, sp, rhs)` succeeds
then, in the resulting metacontext, `Γ ⊢ ?m sp ≡ rhs` (definitionally).

*Proof.* The solution is closed of type `τₘ`; forcing `?m sp` β-reduces it to
`rhs'[xᵢ ↦ sp⟨i⟩]` (each spine entry is applied to its λ), which equals
`rhs` by Theorem 4.6. ∎

### 4.5 Pruning

When the renamed `rhs` contains `?m' t⃗'` whose spine mentions a variable
**not** in `σ`'s domain, `rename` does not fail outright: if the spine is
otherwise a renaming, the meta can be *pruned*.

**Definition 4.9 (pruning).** For a mask `π : k → Bool` over `?m'`'s
arguments (⊤ = keep; ordered outermost-first):

```
pruneTy π (Π (x:A). B) = Π (x:A). pruneTy π' B      (π = ⊤ ∷ π′)
pruneTy π (Π (x:A). B) = pruneTy π′ B               (π = ⊥ ∷ π′)
pruneTy []      B      = B′  (B renamed under the accumulated σ; fails
                              if a dropped binder occurs in it)
pruneMeta π m' = fresh m″ : pruneTy π τₘ′ ;  Δ(m′) := λ x⃗. m″ (x⃗ ↾ π)
```

**Lemma 4.10 (side condition).** `pruneTy π τ` succeeds iff the pruned type
is well-formed: the Π-chain is at least as long as `π`, and after dropping
the `⊥` binders no remaining subterm mentions them (the renaming in the
codomain is exactly a scope check). ∎

**Theorem 4.11 (pruned solution preserves the equation).** Suppose
`rename` meets `?m' t⃗'` with pruning mask `π`, producing `?m″ t⃗″`, and that
`pruneTy π τₘ′` succeeds (Lemma 4.10). Then after
`Δ(m′) := λ x⃗. m″ (x⃗ ↾ π)`:

```
?m' t⃗'  ≡  ?m″ t⃗″
```

in any context where both type-check, and hence any equation that renamed to
one containing `?m″ t⃗″` still holds after substitution of both solutions.

*Proof.* The solution's λ-telescope has the arity of `τₘ′`; forcing `?m' t⃗'`
applies it to the actual arguments and projects: `λ x⃗. m″ (x⃗ ↾ π) t⃗'`β→
`m″ (t⃗' ↾ π)` — the kept arguments in order — which is `t⃗″` by the
construction of the renamed spine (kept entries rename to exactly these
terms). The scope discipline of `t⃗″` is Lemma 4.10's. ∎

**Remark 4.12 (the failed-prune case).** Without Lemma 4.10's side
condition the solution would be ill-typed — the zoo's second example,
`m : (A B : U) → A → B → B` under `m a a ≐ λ x y. y`, is the witness: the
codomain mentions the dropped binders, `pruneTy` fails, and unification
reports mismatch rather than recording a scoping-incorrect solution.

### 4.6 Intersection and non-linear spines

**Definition 4.13 (intersection).** `?m sp ≐ ?m sp′` with both spines
renamings solves by `π i = ⊤` iff `sp⟨i⟩ = sp′⟨i⟩`, then `pruneMeta π`.

**Proposition 4.14.** After the pruning, both sides reduce to
`?m″ (sp ↾ π) = ?m″ (sp′ ↾ π)` with `sp ↾ π = sp′ ↾ π` (the kept positions
are exactly the agreeing ones). ∎

**Remark 4.15 (shape, not value).** Intersection fixes the *shape*
`λ x y z. m″ y` but not `m″`; the zoo reports the residual unsolved meta, the
port reports `unsolved metavariable ?{m}` at the hole (its pruned type in
the metacontext).

**Definition 4.16 (non-linear spines).** Solving `?m sp ≐ rhs` with repeated
spine variables proceeds as in §4.4 *provided* `pruneTy ν τₘ` succeeds, where
`ν` is `invert`'s non-linearity mask; the solution abstracts all `k`
arguments and the duplicated ones are projected away exactly as in
Theorem 4.11.

**Theorem 4.17.** The side condition is necessary: `m : (A B : U) → A → B → B`
under `m a a ≐ λ x y. y` fails it, and no well-typed solution of the shape
`λ … . rhs′` with `rhs′` not using `a` exists. ∎ (Witness calculation.)

### 4.7 The elaboration strategy

The check-position comparison is

```
conv_or_unify_le t u = conv_le(t, u)  ∨  (metas exist? unify(t, u) : false)
```

**Proposition 4.18 (zero cost, cumulativity intact).** For meta-free
programs the fallback is unreachable and the comparison is byte-identical to
the pre-branch behaviour. When metas exist, `conv_le` — hence cumulativity —
is tried first; unification is only consulted for a failure, where no
cumulative subtyping judgement exists to preserve (the meta side is solved
to the rigid side, which is a subtype-sound solution). ∎

`infer` of a variable-only λ-chain creates one domain meta per binder and
consumes the *whole* chain before closing the type with a single quotation —
the term-level counterpart of the lazy type closing below. `check` of `_`
against `A` is `fresh` at `A`; `infer` of `_` is a meta over `U₁`'s elements;
a type-position hole is solved to a `U₀`-level type (`infer_type` unifies the
type-of-type meta with `U`).

**Theorem 4.19 (global soundness, informal).** If elaboration of `P` under
the initial metacontext Δ₀ succeeds with all metas solved, then the resulting
core term is well-typed in 𝒯, and each `unify` step replaced a definitional
equality problem by an equality that holds in the final metacontext.
*Proof sketch.* Induction over the elaboration, with Theorems 4.8 and 4.11
discharging the `unify` steps; failure paths abort the elaboration (partial
metacontexts are never observed, hence the conservative *not sure* of the
`refl` index unifier when it meets metas).

### 4.8 Termination and the implementation

**Remark 4.20 (termination).** `unify` terminates: the occurs check forbids
cyclic solutions; every recursive call either descends structurally or, for
the η-step on a stuck λ, applies *both* sides to a fresh variable — after
which a variable-pattern λ consumes the argument and a `refl`/`(!)`-pattern
λ would re-block forever. This is why η in `unify` is restricted to
variable-pattern λs; pattern λs fall back to structural conversion.

**Remark 4.21 (lazy type closing).** The port stores meta types *open* — the
value in its context plus the shared, once-quoted context chain (the zoo's
`Locals`; defines close to `Let`s, which is what keeps de Bruijn indices
aligned) — and closes them only for pruning, `lams`, and the unsolved
report. Closing eagerly re-quotes the context per binder: measured 12 s at
160 nested λs (cubic-plus) against 0.1 s after (quadratic).

**Remark 4.22 (an index-alignment bug, post mortem).** The closed type
contains `Let` nodes for defines; an early `lams` returned the body un-wrapped
when it met a `Let`, producing a solution without its λ-telescope. The first
spine application of such a solution explodes (`U` applied to an argument).
Both `lams` and `pruneTy` now walk under `Let`s — the general lesson is
Invariant 4.3 stated once and checked at every producer of indices.

## 5. What the Lean files formalize

Companion files: [`lean/Prune.lean`](lean/Prune.lean) (Branch A),
[`lean/PatternUnif.lean`](lean/PatternUnif.lean) (Branch B).

| Statement | Lean name (file) |
|---|---|
| Def 3.3 resolution / Def 3.5 reachability | `resolveIn`, `Reach` (Prune) |
| Lemma 3.4 innermost-provider | `resolveIn_innermost` (Prune) |
| Lemma 3.7 kept providers | `resolve_reach` (Prune) |
| Theorem 3.8, disjointness half | `pruned_never_provides` (Prune) |
| Theorem 3.8, order half | `no_inner_hijack` (Prune) |
| Def 4.2 partial renaming, `lift` | `liftRen`, `liftSp` (PatternUnif) |
| Def 4.4 spine renaming (`invert`'s output, as hypotheses) | `SpinePRen` |
| Invariant 4.3 | `lift_on_spine`, `lift_only_spine`, `lift_inj` |
| Theorem 4.6 inversion | `rename_inversion` |
| Theorem 4.8 solution soundness | `solve_sound` (the β-law of `subst` is definitional here) |

Theorem 4.11's *type-level* side condition (Lemma 4.10) is not formalized:
its content is a scope check, spelled out in the paper proof; its *scope*
half (the pruned solution projects exactly the kept arguments) is the β-law
again and is not restated.

Both files are self-contained Lean 4 (no mathlib), written defensively
(self-defined list plumbing; canonical `finPrev` restriction; deterministic
rewrites along `_fresh`/`_old` equation lemmas only — never an unfold of a
decidable instance).  **Machine-checked**: both compile with zero errors
and zero warnings under Lean 4.35.0 (`lean Prune.lean && lean
PatternUnif.lean`; the toolchain was installed from the release zip
through a range-request mirror after the direct CDN connection proved
throttled).
The formalization is *scope-level*: terms carry context sizes, substitution
is structural, and the β-law is definitional — the type-level layer of 𝒯
(cumulativity, eliminators, `refl` patterns) is not modelled; its
interaction with metas is exactly Theorem 4.19's proof obligation and
remains future work.

## 6. Related work

The `pruning` branch is a port of Kovács' elaboration zoo `05-pruning`
[elaboration-zoo], itself an exposition of Miller-pattern unification with
pruning as in Abél & Pientka's σ-unification survey (§3.4 there generalizes
pruning beyond renaming spines; the port keeps the simple version) [AP16].
The base theory's pattern matching follows Cockx–Devriese–Piessens'
*Pattern Matching Without K* in shape, with-K in behaviour. Declaration
reachability (Branch A) is the classic dead-binding analysis of module
systems; the novelty, if any, is doing it on a linear chain with
elaborator-accurate generated names (Def 3.2's `struct` case) and proving
resolution invariance rather than eyeballing it.

* [elaboration-zoo] A. Kovács, *Elaboration zoo*, 05-pruning.
* [AP16] A. Abél, B. Pientka, *Unification in Dependent Type Theory*.
* [CDP14] J. Cockx, D. Devriese, F. Piessens, *Pattern Matching Without K*.

## 7. Conclusion

Two prunings, two dependency orders, one method: make the scoping invariant
explicit, prove deletion commutes with it. The syntactic branch got a full
elaboration-invariance theorem; the semantic branch got the two lemmas that
carry all of pattern unification's correctness weight — renaming inverts
spine substitution, and pruning is exactly the deletion the meta's type
tolerates. Everything scoping-critical is machine-checked; the remaining gap
is the definitional-equality layer of 𝒯, whose interaction with metas is
well-trodden but unverified here.
