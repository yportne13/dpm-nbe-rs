/-!
# Branch B (`pruning`): the inversion theorem of pattern unification

Formalization of `docs/paper.md` §4 at the *scope* level: well-scoped de
Bruijn terms with metavariable heads, partial renamings, and the theorem the
whole algorithm rests on —

* `rename_inversion` — Theorem 4.6: renaming a use-site term under the
  partial renaming induced by an injective variable spine produces a term of
  the meta's context that, substituted with the spine back, *is* the
  original term.  (Solution soundness, Theorem 4.8, is this statement read
  through the β-law of `subst`.)

Variables are de Bruijn *levels* (0 = oldest binder): under `lam` the new
binder is level `n`, which makes the index discipline of lifting (new
use-site level `γ` ↦ new meta level `k`) and the weakening casts trivial —
Invariant 4.3 of the paper, verified here as `lift_on_spine` and
`lift_only_spine`.

Design notes: a metavariable head is a *bare* `mvar g` node (the name
`meta` is a Lean modifier; spines are chains of `app`s around the head),
which keeps `Tm` a plain (non-nested) inductive family.  `finPrev` is the
single canonical restriction `Fin (k+1) → Fin k`, and every proof rewrites
along a `_fresh`/`_old` equation lemma stated for a *variable* input — so
neither decidable-instance unfolding nor proof-term identity is ever
relied on.

Machine-checked with Lean 4.35.0 (no mathlib): `lean PatternUnif.lean`.
-/

/-- Constructor disequalities (stated literally; `injection` on them). -/
theorem none_some_ne {α : Type} {a : α} :
    (none : Option α) = some a → False := by
  intro h; injection h

theorem some_none_ne {α : Type} {a : α} :
    (some a : Option α) = none → False := by
  intro h; injection h

theorem map_some {α β : Type} (f : α → β) (a : α) :
    Option.map f (some a) = some (f a) := rfl

/-- Terms over `n` de Bruijn levels, with bare metavariable heads. -/
inductive Tm : Nat → Type where
  | var : Fin n → Tm n
  | app : Tm n → Tm n → Tm n
  | lam : Tm (n + 1) → Tm n
  | mvar : Nat → Tm n

/-! ## Weakening and substitution -/

/-- The restriction of a `Fin (k+1)` to `Fin k`, with one canonical proof. -/
def finPrev {k : Nat} (i : Fin (k + 1)) (h : ¬(i.1 = k)) : Fin k :=
  ⟨i.1, by have h2 := i.2; omega⟩

/-- Levels are preserved by weakening; only the bound grows. -/
def castTm : {n m : Nat} → n ≤ m → Tm n → Tm m
  | _, _, h, .var i => .var ⟨i.1, Nat.lt_of_lt_of_le i.2 h⟩
  | _, _, h, .app a b => .app (castTm h a) (castTm h b)
  | _, _, h, .lam t => .lam (castTm (Nat.succ_le_succ h) t)
  | _, _, _h, .mvar g => .mvar g

/-- Extend an environment under a binder: the new level `n` maps to the new
    level `m`; old levels are weakened. -/
def liftEnv {n m : Nat} (f : Fin n → Tm m) : Fin (n + 1) → Tm (m + 1) :=
  fun i =>
    if h : i.1 = n then .var ⟨m, Nat.lt_succ_self m⟩
    else castTm (Nat.le_succ m) (f (finPrev i h))

/-- Substitute levels: replace `var i` by `f i`. -/
def subst : {n m : Nat} → (Fin n → Tm m) → Tm n → Tm m
  | _, _, f, .var i => f i
  | _, _, f, .app a b => .app (subst f a) (subst f b)
  | _, _, f, .lam t => .lam (subst (liftEnv f) t)
  | _, _, _f, .mvar g => .mvar g

theorem subst_var {n m : Nat} (f : Fin n → Tm m) (i : Fin n) :
    subst f (.var i) = f i := rfl

theorem subst_app {n m : Nat} (f : Fin n → Tm m) (a b : Tm n) :
    subst f (.app a b) = .app (subst f a) (subst f b) := rfl

theorem subst_lam {n m : Nat} (f : Fin n → Tm m) (t : Tm (n + 1)) :
    subst f (.lam t) = .lam (subst (liftEnv f) t) := rfl

/-! ## Partial renamings -/

/-- Lift a renaming under a binder on both sides: the new use-site level
    (the old `γ`) maps to the new meta level (the old `δ`). -/
def liftRen {γ δ : Nat} (σ : Fin γ → Option (Fin δ)) :
    Fin (γ + 1) → Option (Fin (δ + 1)) :=
  fun i =>
    if h : i.1 = γ then some ⟨δ, Nat.lt_succ_self δ⟩
    else (σ (finPrev i h)).map (fun j => ⟨j.1, Nat.lt_succ_of_lt j.2⟩)

/-- Lift a spine: the new use-site level extends it. -/
def liftSp {γ k : Nat} (sp : Fin k → Fin γ) : Fin (k + 1) → Fin (γ + 1) :=
  fun i =>
    if h : i.1 = k then ⟨γ, Nat.lt_succ_self γ⟩
    else ⟨(sp (finPrev i h)).1, Nat.lt_succ_of_lt (sp (finPrev i h)).2⟩

/-- Rename a use-site term into the meta-context.  `occ` is the
    metavariable being solved (the occurs check); `none` on a scope error
    or an occurs failure. -/
def rename {γ δ : Nat} (occ : Option Nat) (σ : Fin γ → Option (Fin δ)) :
    Tm γ → Option (Tm δ)
  | .var i => (σ i).map .var
  | .app a b =>
    match rename occ σ a, rename occ σ b with
    | some a', some b' => some (.app a' b')
    | _, _ => none
  | .lam t => (rename occ (liftRen σ) t).map .lam
  | .mvar g => if occ = some g then none else some (.mvar g)

theorem rename_var {γ δ : Nat} (occ) (σ : Fin γ → Option (Fin δ)) (i : Fin γ) :
    rename occ σ (.var i) = (σ i).map .var := rfl

theorem rename_app {γ δ : Nat} (occ) (σ : Fin γ → Option (Fin δ)) (a b : Tm γ) :
    rename occ σ (.app a b) =
      (match rename occ σ a, rename occ σ b with
      | some a', some b' => some (.app a' b')
      | _, _ => none) := rfl

theorem rename_lam {γ δ : Nat} (occ) (σ : Fin γ → Option (Fin δ))
      (t : Tm (γ + 1)) :
    rename occ σ (.lam t) = (rename occ (liftRen σ) t).map .lam := rfl

theorem rename_mvar {γ δ : Nat} (occ) (σ : Fin γ → Option (Fin δ)) (g : Nat) :
    rename occ σ (.mvar g) = if occ = some g then none else some (.mvar g) := rfl

/-! ## Equation lemmas for the lifting functions

Stated for a *variable* input (the decidable instance never unfolds) and in
terms of `finPrev` (proof terms stay canonical). -/

theorem liftSp_fresh {γ k : Nat} (sp : Fin k → Fin γ) (i : Fin (k + 1))
    (h : i.1 = k) :
    liftSp sp i = ⟨γ, Nat.lt_succ_self γ⟩ := by
  simp only [liftSp]; rw [dite_eq_left h]

theorem liftSp_old {γ k : Nat} (sp : Fin k → Fin γ) (i : Fin (k + 1))
    (h : ¬(i.1 = k)) :
    liftSp sp i = ⟨(sp (finPrev i h)).1,
      Nat.lt_succ_of_lt (sp (finPrev i h)).2⟩ := by
  simp only [liftSp]; rw [dite_eq_right h]

theorem liftSp_val_fresh {γ k : Nat} (sp : Fin k → Fin γ) (i : Fin (k + 1))
    (h : i.1 = k) : (liftSp sp i).val = γ := by
  rw [liftSp_fresh sp i h]

theorem liftSp_val_old {γ k : Nat} (sp : Fin k → Fin γ) (i : Fin (k + 1))
    (h : ¬(i.1 = k)) : (liftSp sp i).val = (sp (finPrev i h)).val := by
  rw [liftSp_old sp i h]

theorem liftRen_fresh {γ δ : Nat} (σ : Fin γ → Option (Fin δ))
    (i : Fin (γ + 1)) (h : i.1 = γ) :
    liftRen σ i = some ⟨δ, Nat.lt_succ_self δ⟩ := by
  simp only [liftRen]; rw [dite_eq_left h]

theorem liftRen_old {γ δ : Nat} (σ : Fin γ → Option (Fin δ))
    (i : Fin (γ + 1)) (h : ¬(i.1 = γ)) :
    liftRen σ i = (σ (finPrev i h)).map (fun j => ⟨j.1, Nat.lt_succ_of_lt j.2⟩) := by
  simp only [liftRen]; rw [dite_eq_right h]

theorem liftEnv_fresh {n m : Nat} (f : Fin n → Tm m) (i : Fin (n + 1))
    (h : i.1 = n) :
    liftEnv f i = .var ⟨m, Nat.lt_succ_self m⟩ := by
  simp only [liftEnv]; rw [dite_eq_left h]

theorem liftEnv_old {n m : Nat} (f : Fin n → Tm m) (i : Fin (n + 1))
    (h : ¬(i.1 = n)) :
    liftEnv f i = castTm (Nat.le_succ m) (f (finPrev i h)) := by
  simp only [liftEnv]; rw [dite_eq_right h]

/-! ## Spine renamings

The renaming induced by an injective spine, as hypotheses: every spine
variable maps to its slot, and nothing else is mapped.  (`invert` in the
implementation produces exactly this renaming.) -/

structure SpinePRen (γ k : Nat) where
  sp : Fin k → Fin γ
  inj : Function.Injective sp
  σ : Fin γ → Option (Fin k)
  on_spine : ∀ i, σ (sp i) = some i
  only_spine : ∀ v j, σ v = some j → sp j = v

/-- The lifted renaming maps the lifted spine — **Invariant 4.3**, forward
    half. -/
theorem lift_on_spine {γ k : Nat} (s : SpinePRen γ k) (i : Fin (k + 1)) :
    liftRen s.σ (liftSp s.sp i) = some i := by
  by_cases h : i.1 = k
  · rw [liftSp_fresh _ i h, liftRen_fresh _ _ (by rfl)]
    exact congrArg some (Fin.ext h.symm)
  · rw [liftSp_old _ i h,
      liftRen_old _ _ (by
        show (s.sp (finPrev i h)).1 = γ → False
        intro hc
        have hlt := (s.sp (finPrev i h)).2
        omega)]
    show (s.σ (s.sp (finPrev i h))).map
        (fun j => ⟨j.1, Nat.lt_succ_of_lt j.2⟩) = some i
    rw [s.on_spine (finPrev i h)]
    exact congrArg some (Fin.ext rfl)

/-- The lifted renaming is defined only on the lifted spine —
    **Invariant 4.3**, backward half. -/
theorem lift_only_spine {γ k : Nat} (s : SpinePRen γ k)
    (v : Fin (γ + 1)) (j : Fin (k + 1))
    (h : liftRen s.σ v = some j) : liftSp s.sp j = v := by
  by_cases hv : v.1 = γ
  · rw [liftRen_fresh _ v hv] at h
    cases h
    have hfresh : liftSp s.sp ⟨k, Nat.lt_succ_self k⟩
        = ⟨γ, Nat.lt_succ_self γ⟩ :=
      liftSp_fresh _ _ (by rfl)
    rw [hfresh]
    exact Fin.ext hv.symm
  · rw [liftRen_old _ v hv] at h
    cases hσ : s.σ (finPrev v hv) with
    | none =>
      rw [hσ] at h
      exact absurd h none_some_ne
    | some m =>
      rw [hσ, map_some] at h
      cases h
      have hsm : s.sp m = finPrev v hv := s.only_spine (finPrev v hv) m hσ
      have hval : (s.sp m).val = v.val := by rw [hsm]; rfl
      have hmlt : m.1 < k := m.2
      apply Fin.ext
      show (liftSp s.sp ⟨m.1, by omega⟩).val = v.val
      rw [liftSp_val_old _ _ (by show ¬(m.1 = k); omega)]
      show (s.sp m).val = v.val
      exact hval

/-- The lifted spine is injective. -/
theorem lift_inj {γ k : Nat} (s : SpinePRen γ k) :
    Function.Injective (liftSp s.sp) := by
  intro a b hab
  have hval : (liftSp s.sp a).val = (liftSp s.sp b).val := congrArg Fin.val hab
  by_cases ha : a.1 = k
  · by_cases hb : b.1 = k
    · rw [liftSp_val_fresh _ a ha, liftSp_val_fresh _ b hb] at hval
      exact Fin.ext (by omega)
    · rw [liftSp_val_fresh _ a ha, liftSp_val_old _ b hb] at hval
      have h2 := (s.sp (finPrev b hb)).2
      exact absurd hval (by omega)
  · by_cases hb : b.1 = k
    · rw [liftSp_val_old _ a ha, liftSp_val_fresh _ b hb] at hval
      have h2 := (s.sp (finPrev a ha)).2
      exact absurd hval.symm (by omega)
    · rw [liftSp_val_old _ a ha, liftSp_val_old _ b hb] at hval
      have heq : s.sp (finPrev a ha) = s.sp (finPrev b hb) := Fin.ext hval
      have hinj := s.inj heq
      have hfin : a.1 = b.1 := by
        have h1 : (finPrev a ha).val = (finPrev b hb).val :=
          congrArg Fin.val hinj
        show a.1 = b.1
        exact h1
      exact Fin.ext (by omega)

/-! ## The inversion theorem -/

/-- `liftEnv` of a spine environment is the environment of the lifted
    spine — the alignment that makes the `lam` case work. -/
theorem liftEnv_var_spine {γ k : Nat} (sp : Fin k → Fin γ) :
    (liftEnv (fun i => Tm.var (sp i))) = (fun i => Tm.var (liftSp sp i)) := by
  funext i
  by_cases h : i.1 = k
  · rw [liftEnv_fresh _ i h, liftSp_fresh _ i h]
  · rw [liftEnv_old _ i h, liftSp_old _ i h]
    exact congrArg Tm.var (Fin.ext rfl)

/-- **Theorem 4.6 (inversion).**  Renaming a use-site term under an
    injective spine's renaming inverts spine substitution: the renamed term,
    with the spine substituted back, *is* the original. -/
theorem rename_inversion {n : Nat} (t : Tm n) :
    ∀ {k : Nat} (s : SpinePRen n k) (t' : Tm k),
      rename none s.σ t = some t' →
      subst (fun i => Tm.var (s.sp i)) t' = t := by
  induction t with
  | var v =>
    intro k s t' h
    rw [rename_var] at h
    cases hv : s.σ v with
    | none => rw [hv] at h; exact absurd h none_some_ne
    | some j =>
      rw [hv, map_some] at h
      cases h
      show Tm.var (s.sp j) = Tm.var v
      rw [s.only_spine v j hv]
  | app a b iha ihb =>
    intro k s t' h
    rw [rename_app] at h
    match ha : rename none s.σ a, hb : rename none s.σ b with
    | some a', some b' =>
      rw [ha, hb] at h
      cases h
      rw [subst_app, iha s a' ha, ihb s b' hb]
    | some _, none => rw [ha, hb] at h; exact absurd h none_some_ne
    | none, some _ => rw [ha, hb] at h; exact absurd h none_some_ne
    | none, none => rw [ha, hb] at h; exact absurd h none_some_ne
  | lam t ih =>
    intro k s t' h
    rw [rename_lam] at h
    cases ht : rename none (liftRen s.σ) t with
    | none => rw [ht] at h; exact absurd h none_some_ne
    | some t1' =>
      rw [ht, map_some] at h
      cases h
      -- the lifted renaming is a spine renaming of the lifted spine
      -- the lifted renaming, inlined so its projections reduce, is a
      -- spine renaming of the lifted spine
      have ih' := ih
        (s := ⟨liftSp s.sp, lift_inj s, liftRen s.σ,
            lift_on_spine s, lift_only_spine s⟩) t1' ht
      -- and the lifted environment is exactly the lifted spine's
      rw [subst_lam, liftEnv_var_spine]
      exact congrArg Tm.lam ih'
  | mvar g =>
    intro k s t' h
    rw [rename_mvar, ite_eq_right (none_some_ne)] at h
    cases h
    rfl

/-- **Theorem 4.8 (solution soundness), scope reading.**  With the solution
    `λ⃗. rhs'` recorded for `?m`, forcing `?m sp` applies the telescope and
    substitutes the spine — the β-law of `subst`, definitional here — which
    by `rename_inversion` is the identity on the right-hand side. -/
theorem solve_sound {γ k : Nat} (s : SpinePRen γ k) (rhs : Tm γ)
    (rhs' : Tm k) (hr : rename none s.σ rhs = some rhs') :
    subst (fun i => Tm.var (s.sp i)) rhs' = rhs :=
  rename_inversion rhs s rhs' hr
