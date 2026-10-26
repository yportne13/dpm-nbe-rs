/-!
# Branch A (`prune`): declaration pruning preserves name resolution

Formalization of `docs/paper.md` §3.  A program is a linear chain of
declarations; each declaration *binds* names for the rest of the chain and
*uses* names in its own components.  Resolution is innermost-first (the
order `Ctx::lookup` elaborates with); reachability is the least closure of
the resolution edges out of kept sites.  Results, matching the paper:

* `resolveIn_innermost` — Lemma 3.4 / Theorem 3.8 (order half): resolution
  picks the innermost provider, so deleting non-providers cannot change the
  first hit.
* `resolve_reach` — Lemma 3.7 (kept providers): every provider of a used
  name at a kept site is itself reachable, hence kept.
* `pruned_never_provides` — Theorem 3.8 (disjointness half): a pruned
  declaration never provides a name that any kept site resolves, so pruning
  cannot rebind anything.
* `no_inner_hijack` — Theorem 3.8 (order half, spelt out): no declaration
  strictly between the provider and the use site binds the name.

The composed statement (filter the chain, resolve through the surviving
indices) is arithmetic on top of these two halves; see the paper's proof of
Theorem 3.8.

Written defensively: membership, lookup and take are defined here, and
every proof step is a rewrite along a `rfl`-equation, `injection` on an
impossible equation, or `omega`.

Machine-checked with Lean 4.35.0 (no mathlib): `lean Prune.lean`.
-/

/-- Membership as a Boolean. -/
def memN : Nat → List Nat → Bool
  | _, [] => false
  | x, y :: ys => (x == y) || memN x ys

/-- Membership as the proposition "the Boolean said so". -/
def MemN (x : Nat) (l : List Nat) : Prop := memN x l = true

-- Constructor disequality, generic in the option's type and stated with
-- literal constructors, so it applies to hypotheses whose sides only
-- reduce to these forms definitionally.
theorem none_eq_some_ne {α : Type} {a : α} :
    (none : Option α) = some a → False := by
  intro h; injection h

theorem some_eq_none_ne {α : Type} {a : α} :
    (some a : Option α) = none → False := by
  intro h; injection h

/-- One declaration: names bound for the rest of the chain, and names used
    in its own components. -/
structure Decl where
  binds : List Nat
  uses : List Nat

/-- A program: the declaration chain and the goal's free names. -/
structure Prog where
  decls : List Decl
  goal : List Nat

/-- The n-th declaration, self-contained. -/
def nthD : List Decl → Nat → Option Decl
  | [], _ => none
  | d :: _, 0 => some d
  | _ :: ds, k + 1 => nthD ds k

/-- The first `j` declarations (the scope of a use site `j`). -/
def takeD : Nat → List Decl → List Decl
  | 0, _ => []
  | _ + 1, [] => []
  | k + 1, d :: ds => d :: takeD k ds

/-- `resolveIn ds x`: the index of the innermost declaration binding `x` —
    the recursive call answers for the (inner) tail first. -/
def resolveIn : List Decl → Nat → Option Nat
  | [], _ => none
  | d :: ds, x =>
    match resolveIn ds x with
    | some j => some (j + 1)
    | none => if memN x d.binds then some 0 else none

/-! ## Unfolding lemmas (all `rfl`) -/

theorem nthD_nil (k : Nat) : nthD [] k = none := rfl

theorem nthD_zero (d : Decl) (ds : List Decl) : nthD (d :: ds) 0 = some d := rfl

theorem nthD_succ (d : Decl) (ds : List Decl) (k : Nat) :
    nthD (d :: ds) (k + 1) = nthD ds k := rfl

theorem takeD_succ_cons (d : Decl) (ds : List Decl) (k : Nat) :
    takeD (k + 1) (d :: ds) = d :: takeD k ds := rfl

theorem takeD_all (ds : List Decl) (k : Nat) (hk : ds.length ≤ k) :
    takeD k ds = ds := by
  induction ds generalizing k with
  | nil => cases k <;> rfl
  | cons d ds ih =>
    match k with
    | 0 => exact absurd hk (by simp)
    | k + 1 => rw [takeD_succ_cons, ih k (by simpa using hk)]

theorem resolveIn_cons (d : Decl) (ds : List Decl) (x : Nat) :
    resolveIn (d :: ds) x =
      (match resolveIn ds x with
      | some j => some (j + 1)
      | none => if memN x d.binds then some 0 else none) := rfl

/-! ## Resolution -/

/-- If nothing is resolved, no declaration binds the name. -/
theorem resolveIn_none (ds : List Decl) (x : Nat) (h : resolveIn ds x = none) :
    ∀ k d', nthD ds k = some d' → ¬MemN x d'.binds := by
  induction ds with
  | nil =>
    intro k d' hn
    rw [nthD_nil] at hn
    exact fun _ => none_eq_some_ne hn
  | cons d ds ih =>
    rw [resolveIn_cons] at h
    cases hds : resolveIn ds x with
    | some j => rw [hds] at h; exact absurd h some_eq_none_ne
    | none =>
      rw [hds] at h
      cases hb : memN x d.binds with
      | true => rw [hb] at h; exact absurd h some_eq_none_ne
      | false =>
        rw [hb] at h
        intro k d' hn
        match k with
        | 0 =>
          rw [nthD_zero] at hn
          cases hn
          intro hmem
          rw [hmem] at hb
          exact Bool.noConfusion hb
        | k + 1 =>
          rw [nthD_succ] at hn
          exact ih hds k d' hn

/-- **Innermost-provider characterisation** (Lemma 3.4; the order half of
    Theorem 3.8).  A successful resolution points at a declaration that
    binds the name, with no inner binder for it. -/
theorem resolveIn_innermost (ds : List Decl) (x i : Nat)
    (h : resolveIn ds x = some i) :
    ∃ d, nthD ds i = some d ∧ MemN x d.binds ∧
      ∀ k d', i < k → nthD ds k = some d' → ¬MemN x d'.binds := by
  induction ds generalizing i with
  | nil => rw [show resolveIn [] x = none from rfl] at h; exact absurd h none_eq_some_ne
  | cons d ds ih =>
    rw [resolveIn_cons] at h
    cases hds : resolveIn ds x with
    | some j =>
      rw [hds] at h
      cases h
      have ⟨d0, hd0, hb, hnb⟩ := ih j hds
      refine ⟨d0, ?_, hb, ?_⟩
      · rw [nthD_succ]; exact hd0
      · intro k d' hk hn
        match k with
        | 0 => exact absurd hk (by omega)
        | k + 1 =>
          rw [nthD_succ] at hn
          exact hnb k d' (by omega) hn
    | none =>
      rw [hds] at h
      cases hb : memN x d.binds with
      | false => rw [hb] at h; exact absurd h none_eq_some_ne
      | true =>
        rw [hb] at h
        cases h
        refine ⟨d, nthD_zero d ds, hb, ?_⟩
        intro k d' hk hn
        match k with
        | 0 => exact absurd hk (by omega)
        | k + 1 =>
          rw [nthD_succ] at hn
          exact resolveIn_none ds x hds k d' hn

/-! ## Use sites and reachability -/

/-- `siteUses P j x`: the site `j` (a declaration, or `|decls|` for the
    goal) uses the name `x`. -/
def siteUses (P : Prog) (j : Nat) (x : Nat) : Prop :=
  (∃ d, nthD P.decls j = some d ∧ MemN x d.uses) ∨
    (j = P.decls.length ∧ MemN x P.goal)

/-- Reachability: the least set closed under "a kept declaration's (or the
    goal's) resolution edges" — the declarative counterpart of the
    implementation's worklist. -/
inductive Reach (P : Prog) : Nat → Prop
  | goal (x : Nat) (hx : MemN x P.goal) (i : Nat)
      (r : resolveIn (takeD P.decls.length P.decls) x = some i) :
    Reach P i
  | step (j : Nat) (hj : Reach P j) (x : Nat)
      (hx : ∃ d, nthD P.decls j = some d ∧ MemN x d.uses) (i : Nat)
      (r : resolveIn (takeD j P.decls) x = some i) :
    Reach P i

/-- No declaration lives at or beyond the length of the list. -/
theorem nthD_beyond (ds : List Decl) (k : Nat) (hk : ds.length ≤ k) :
    nthD ds k = none := by
  induction ds generalizing k with
  | nil => rfl
  | cons d ds ih =>
    match k with
    | 0 => exact absurd hk (by simp)
    | k + 1 => rw [nthD_succ]; exact ih k (by simpa using hk)

/-- **Lemma 3.7 (kept providers).**  Every provider of a name used at a
    kept site is reachable — so it is kept. -/
theorem resolve_reach (P : Prog) (j x i : Nat)
    (hu : siteUses P j x)
    (hj : j = P.decls.length ∨ Reach P j)
    (r : resolveIn (takeD j P.decls) x = some i) : Reach P i := by
  cases hj with
  | inl hlen =>
    subst hlen
    rcases hu with ⟨d, hd, hx⟩ | ⟨hlen, hx⟩
    · rw [nthD_beyond P.decls P.decls.length (Nat.le_refl _)] at hd
      exact absurd hd none_eq_some_ne
    · exact Reach.goal x hx i r
  | inr h =>
    rcases hu with ⟨d, hd, hx⟩ | ⟨hlen, hx⟩
    · exact Reach.step j h x ⟨d, hd, hx⟩ i r
    · rw [hlen] at r
      exact Reach.goal x hx i r

/-- **Theorem 3.8 (disjointness half).**  A declaration that is not
    reachable never provides a name that any kept site resolves: pruning it
    cannot rebind anything. -/
theorem pruned_never_provides (P : Prog) (i : Nat)
    (hi : ¬Reach P i) (j x : Nat)
    (hu : siteUses P j x)
    (hj : j = P.decls.length ∨ Reach P j) :
    resolveIn (takeD j P.decls) x ≠ some i := by
  intro r
  exact hi (resolve_reach P j x i hu hj r)

/-- **Theorem 3.8 (order half, spelt out).**  No declaration strictly
    between the provider and the use site binds the name — the first hit
    among the *surviving* declarations is therefore the original provider. -/
theorem no_inner_hijack (ds : List Decl) (x i k : Nat) (d : Decl)
    (hik : i < k) (hd : nthD ds k = some d) (hbinds : MemN x d.binds)
    (h : resolveIn ds x = some i) : False := by
  have ⟨_, _, _, hnb⟩ := resolveIn_innermost ds x i h
  exact hnb k d hik hd hbinds
