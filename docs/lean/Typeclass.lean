/-!
# Typeclasses: resolution by unification, with snapshot/rollback (deviation 20)

Formalization of the instance machinery at the *scope* level, in the
style of the companion files.  The model mirrors `tc.rs`:

* `Tt` has the three shapes resolution touches: a *parameter* (`prm i` —
  the fresh metavariables an instance's telescope becomes), a rigid node
  (`nd c as` — a class application), and application (`app` — the shape
  of resolved dictionary terms).
* The metacontext is a function `Nat → Option Tt`; `mbind` extends it
  (only ever at an unsolved parameter, which is what makes it monotone).
* `unify` is first-order matching with binding — the specialization of
  the deviation-18 unifier to rigid-vs-parameter problems.  A failed
  match may have already bound parameters: the mutation the real
  implementation leaves in its thread-local store, and the reason
  `resolve` keeps a snapshot.
* `tryEntry` walks an entry's telescope: the codomain unifies against
  the target (binding the parameters), ordinary domains must come out
  solved (they become the instance's arguments), and each class domain
  is resolved recursively with the remaining fuel.
* `resolve` scans the table newest-first, and — the discipline this file
  pins — a failed entry's metacontext is discarded: the next attempt
  proceeds from the caller's snapshot (`tc.rs`'s `meta::restore`).

The theorems:

* `unify_sound` — a successful unification (and list walk) only ever
  adds solutions.
* `resolve_extends` — through the whole search (entry try, domain walk,
  recursive subgoal), success still extends.  Combined with the snapshot
  discipline (a failed `resolve` returns no metacontext, and each failed
  entry restarts from the caller's), no partial solution is ever
  observed.
* `walkDoms_shape` / `resolve_shape` — a successful resolution is
  *some* entry's term applied to a list of arguments: the structural
  form of "the found dictionary is an instance application".
* The sanity examples mirror the probe matrix: a direct entry resolves;
  a parameterized entry with a class constraint resolves through it; no
  matching entry fails; a later entry shadows an earlier one.

The `Nat` fuel is the same totality artifact as in `Termination.lean`.
Machine-checked with Lean 4.35.0 (no mathlib): `lean Typeclass.lean`.
-/

set_option linter.unusedVariables false
set_option linter.deprecated false

/-- Constructor disequalities (stated literally; `injection` on them). -/
theorem none_some_ne {a : Tt} : (none : Option Tt) = some a → False := by
  intro h; injection h

theorem some_none_ne {a : Tt} : (some a : Option Tt) = none → False := by
  intro h; injection h

inductive Tt : Type where
  | prm : Nat → Tt
  | nd : Nat → List Tt → Tt
  | app : Tt → Tt → Tt
  deriving BEq

/-- The metacontext: parameter id ↦ solution. -/
abbrev MCtx := Nat → Option Tt

/-- Extend the metacontext with one solution (at a parameter known to be
    unsolved). -/
def mbind (m : MCtx) (i : Nat) (t : Tt) : MCtx := fun j => if j = i then some t else m j

/-- `m'` extends `m`: solutions are only ever added, never changed. -/
def Extends (m m' : MCtx) : Prop := ∀ j t, m j = some t → m' j = some t

theorem ext_refl (m : MCtx) : Extends m m := fun j t h => h

theorem ext_trans {m1 m2 m3 : MCtx} (h1 : Extends m1 m2) (h2 : Extends m2 m3) :
    Extends m1 m3 := fun j t h => h2 j t (h1 j t h)

/-- Binding at an unsolved parameter is monotone. -/
theorem mbind_extends {m : MCtx} {i : Nat} (hnone : m i = none) (t : Tt) :
    Extends m (mbind m i t) := by
  intro j t' h
  show (if j = i then some t else m j) = some t'
  by_cases hj : j = i
  · subst hj
    rw [hnone] at h
    exact absurd h none_some_ne
  · show (if j = i then some t else m j) = some t'
    simp only [hj, if_false]
    exact h

mutual

/-- First-order matching with binding. -/
def unify : Nat → MCtx → Tt → Tt → Option MCtx
  | 0, _, _, _ => none
  | _ + 1, m, .prm i, t =>
    match m i with
    | none => some (mbind m i t)
    | some t' =>
      match t' == t with
      | true => some m
      | false => none
  | f + 1, m, t, p@(.prm _) => unify f m p t
  | f + 1, m, .nd c as, .nd c' bs =>
    if c = c' ∧ as.length = bs.length then unifyList f m as bs else none
  | _, _, _, _ => none

def unifyList : Nat → MCtx → List Tt → List Tt → Option MCtx
  | 0, _, _, _ => none
  | _ + 1, m, [], [] => some m
  | f + 1, m, a :: as, b :: bs =>
    match unify f m a b with
    | some m' => unifyList f m' as bs
    | none => none
  | _, _, _, _ => none

end

/-- Unification is monotone in the metacontext (term and list walk). -/
theorem unify_sound (f : Nat) :
    (∀ (m : MCtx) (a b : Tt) (m' : MCtx), unify f m a b = some m' → Extends m m')
    ∧ (∀ (m : MCtx) (as bs : List Tt) (m' : MCtx),
        unifyList f m as bs = some m' → Extends m m') := by
  induction f with
  | zero =>
    refine ⟨?_, ?_⟩
    · intro m a b m' h; cases h
    · intro m as bs m' h; cases h
  | succ f ih =>
    obtain ⟨ih1, ih2⟩ := ih
    refine ⟨?_, ?_⟩
    · intro m a b m' h
      cases a with
      | prm i =>
        cases hi : m i with
        | none =>
          simp only [unify, hi] at h
          cases h
          exact mbind_extends hi b
        | some t' =>
          simp only [unify, hi] at h
          cases hb : t' == b with
          | true => simp only [hb] at h; cases h; exact ext_refl m
          | false => simp only [hb] at h; cases h
      | nd c as =>
        cases b with
        | prm j =>
          simp only [unify] at h
          exact ih1 m (.prm j) (.nd c as) m' h
        | nd c' bs =>
          simp only [unify] at h
          by_cases hc : c = c' ∧ as.length = bs.length
          · rw [if_pos hc] at h
            exact ih2 m as bs m' h
          · rw [if_neg hc] at h; cases h
        | app x y => cases h
      | app x y =>
        cases b with
        | prm j =>
          simp only [unify] at h
          exact ih1 m (.prm j) (.app x y) m' h
        | nd c' bs => cases h
        | app x' y' => cases h
    · intro m as bs m' h
      cases as with
      | nil =>
        cases bs with
        | nil => cases h; exact ext_refl m
        | cons b bs => cases h
      | cons a as2 =>
        cases bs with
        | nil => cases h
        | cons b bs2 =>
          simp only [unifyList] at h
          cases hu : unify f m a b with
          | none => simp only [hu] at h; cases h
          | some m1 =>
            simp only [hu] at h
            exact ext_trans (ih1 m a b m1 hu) (ih2 m1 as2 bs2 m' h)

/-- Substitute solved parameters into a term. -/
def inst : Nat → MCtx → Tt → Tt
  | 0, _, t => t
  | f + 1, m, .prm i =>
    match m i with
    | some t => inst f m t
    | none => .prm i
  | f + 1, m, .nd c as => .nd c (as.map (inst f m))
  | f + 1, m, .app g a => .app (inst f m g) (inst f m a)

/-- One instance-table entry: a term, its telescope of domains (flagged
    class or ordinary), and the class-application codomain. -/
structure Entry where
  term : Tt
  doms : List (Bool × Tt)
  cod : Tt

/-- The table is scanned head-first, so the head is the newest entry. -/
abbrev Table := List Entry

mutual

/-- Try one entry: unify the codomain against the target, then walk the
    domains. -/
def tryEntry : Nat → MCtx → Entry → Tt → Table → Option (Tt × MCtx)
  | 0, _, _, _, _ => none
  | f + 1, m, e, t, tbl =>
    match unify f m e.cod t with
    | none => none
    | some m1 => walkDoms f m1 e.term e.doms m1 tbl

/-- Walk the domains: ordinary (parameter) domains must be solved and
    become arguments; class domains are resolved recursively, their
    solutions the subgoal dictionaries. -/
def walkDoms : Nat → MCtx → Tt → List (Bool × Tt) → MCtx → Table →
    Option (Tt × MCtx)
  | 0, _, _, _, _, _ => none
  | _ + 1, m, acc, [], _, _ => some (acc, m)
  | f + 1, m, acc, (false, .prm i) :: rest, msub, tbl =>
    match msub i with
    | some x => walkDoms f m (.app acc x) rest msub tbl
    | none => none
  | _ + 1, _, _, (false, _) :: _, _, _ => none
  | f + 1, m, acc, (true, d) :: rest, msub, tbl =>
    match resolve f m (inst f msub d) tbl with
    | none => none
    | some (sg, m3) => walkDoms f m3 (.app acc sg) rest msub tbl

/-- Scan the table newest-first; a failed entry's metacontext is
    discarded — the next attempt starts from the caller's `m`. -/
def resolve : Nat → MCtx → Tt → Table → Option (Tt × MCtx)
  | 0, _, _, _ => none
  | _ + 1, _, _, [] => none
  | f + 1, m, t, e :: es =>
    match tryEntry f m e t es with
    | some r => some r
    | none => resolve f m t es

end

/-- `tryEntry` on a successful codomain match is the domain walk. -/
theorem tryEntry_step (f : Nat) (m : MCtx) (e : Entry) (t : Tt) (tbl : Table)
    (m2 : MCtx) (h : unify f m e.cod t = some m2) :
    tryEntry (f + 1) m e t tbl = walkDoms f m2 e.term e.doms m2 tbl := by
  simp only [tryEntry, h]

/-! ## Success extends -/

/-- **Monotone search.**  A successful resolution extends the
    metacontext — through the codomain match, the domain walk, and every
    recursive subgoal. -/
theorem resolve_extends (f : Nat) :
    (∀ (m : MCtx) (e : Entry) (t : Tt) (tbl : Table) (r : Tt) (m' : MCtx),
        tryEntry f m e t tbl = some (r, m') → Extends m m')
    ∧ (∀ (m : MCtx) (acc : Tt) (doms : List (Bool × Tt)) (msub : MCtx)
        (tbl : Table) (r : Tt) (m' : MCtx),
        walkDoms f m acc doms msub tbl = some (r, m') → Extends m m')
    ∧ (∀ (m : MCtx) (t : Tt) (tbl : Table) (r : Tt) (m' : MCtx),
        resolve f m t tbl = some (r, m') → Extends m m') := by
  induction f with
  | zero =>
    refine ⟨?_, ?_, ?_⟩
    · intro _ _ _ _ _ _ h; cases h
    · intro _ _ _ _ _ _ _ h; cases h
    · intro _ _ _ _ _ h; cases h
  | succ f ih =>
    obtain ⟨ihT, ihW, ihR⟩ := ih
    refine ⟨?_, ?_, ?_⟩
    · intro m e t tbl r m' h
      cases hu : unify f m e.cod t with
      | none => simp only [tryEntry, hu] at h; cases h
      | some m1 =>
        simp only [tryEntry, hu] at h
        exact ext_trans ((unify_sound f).1 m e.cod t m1 hu)
          (ihW m1 e.term e.doms m1 tbl r m' h)
    · intro m acc doms msub tbl r m' h
      match doms with
      | [] => cases h; exact ext_refl m
      | (fl, g) :: rest =>
        match fl, g with
        | false, .prm i =>
          cases hx : msub i with
          | none => simp only [walkDoms, hx] at h; cases h
          | some x =>
            simp only [walkDoms, hx] at h
            exact ihW m (.app acc x) rest msub tbl r m' h
        | false, .nd _ _ => cases h
        | false, .app _ _ => cases h
        | true, d =>
          cases hr : resolve f m (inst f msub d) tbl with
          | none => simp only [walkDoms, hr] at h; cases h
          | some p =>
            simp only [walkDoms, hr] at h
            obtain ⟨sg, m3⟩ := p
            exact ext_trans (ihR m (inst f msub d) tbl sg m3 hr)
              (ihW m3 (.app acc sg) rest msub tbl r m' h)
    · intro m t tbl r m' h
      match tbl with
      | [] => cases h
      | e :: es =>
        cases ht : tryEntry f m e t es with
        | none => simp only [resolve, ht] at h; exact ihR m t es r m' h
        | some p =>
          simp only [resolve, ht] at h
          obtain ⟨r1, m1⟩ := p
          injection h with hp
          injection hp with h1 h2
          subst h1
          subst h2
          exact ihT m e t es r1 m1 ht

/-! ## The shape of a resolved dictionary -/

/-- **Shape.**  The domain walk returns the accumulated term applied to
    the chosen arguments. -/
theorem walkDoms_shape (f : Nat) :
    ∀ (m : MCtx) (acc : Tt) (doms : List (Bool × Tt)) (msub : MCtx)
      (tbl : Table) (r : Tt) (m' : MCtx),
    walkDoms f m acc doms msub tbl = some (r, m') →
      ∃ args : List Tt, r = args.foldl Tt.app acc := by
  induction f with
  | zero => intro m acc doms msub tbl r m' h; cases h
  | succ f ih =>
    intro m acc doms msub tbl r m' h
    match doms with
    | [] => cases h; exact ⟨[], rfl⟩
    | (fl, g) :: rest =>
      match fl, g with
      | false, .prm i =>
        cases hx : msub i with
        | none => simp only [walkDoms, hx] at h; cases h
        | some x =>
          simp only [walkDoms, hx] at h
          obtain ⟨args, hargs⟩ := ih m (.app acc x) rest msub tbl r m' h
          exact ⟨x :: args, by rw [List.foldl_cons]; exact hargs⟩
      | false, .nd _ _ => cases h
      | false, .app _ _ => cases h
      | true, d =>
        cases hr : resolve f m (inst f msub d) tbl with
        | none => simp only [walkDoms, hr] at h; cases h
        | some p =>
          simp only [walkDoms, hr] at h
          obtain ⟨sg, m3⟩ := p
          obtain ⟨args, hargs⟩ := ih m3 (.app acc sg) rest msub tbl r m' h
          exact ⟨sg :: args, by rw [List.foldl_cons]; exact hargs⟩

/-- **A resolved dictionary is an instance application.** -/
theorem resolve_shape (f : Nat) :
    ∀ (m : MCtx) (t : Tt) (tbl : Table) (r : Tt) (m' : MCtx),
    resolve f m t tbl = some (r, m') →
      ∃ e ∈ tbl, ∃ args : List Tt, r = args.foldl Tt.app e.term := by
  induction f with
  | zero => intro m t tbl r m' h; cases h
  | succ f ih =>
    intro m t tbl r m' h
    match tbl with
    | [] => cases h
    | e :: es =>
      cases ht : tryEntry f m e t es with
      | none =>
        simp only [resolve, ht] at h
        obtain ⟨e2, he2, args, hargs⟩ := ih m t es r m' h
        exact ⟨e2, List.mem_cons_of_mem _ he2, args, hargs⟩
      | some p =>
        simp only [resolve, ht] at h
        obtain ⟨r1, m1⟩ := p
        injection h with hp
        injection hp with h1 h2
        subst h1
        subst h2
        -- the fuel is a bare variable; tryEntry only reduces on a
        -- successor (or zero, which contradicts ht)
        match hFuel : f with
        | 0 =>
          rw [show tryEntry 0 m e t es = none from rfl] at ht
          exact absurd ht none_some_ne
        | f2 + 1 =>
          cases hu : unify f2 m e.cod t with
          | none =>
            rw [show tryEntry (f2 + 1) m e t es = none from by
                simp only [tryEntry, hu]] at ht
            exact absurd ht none_some_ne
          | some m2 =>
            rw [show tryEntry (f2 + 1) m e t es
                  = walkDoms f2 m2 e.term e.doms m2 es from by
                simp only [tryEntry, hu]] at ht
            obtain ⟨args, hargs⟩ :=
              walkDoms_shape f2 m2 e.term e.doms m2 es r1 m1 ht
            exact ⟨e, by simp, args, hargs⟩

/-! ## Sanity: the probe matrix, in the model -/

/-- The class head `0`; targets and dictionaries are its applications. -/
private def cls (args : List Tt) : Tt := .nd 0 args

private def m0 : MCtx := fun _ => none

/-- A direct instance resolves. -/
example :
    (resolve 8 m0 (cls [.nd 1 []])
        [{ term := .nd 2 [], doms := [], cod := cls [.nd 1 []] }]).map Prod.fst
      = some (.nd 2 []) := by
  rfl

/-- A parameterized instance with a class constraint resolves through
    the subgoal: the dictionary is the base instance's term, found by
    the recursive search of the constraint. -/
private def eqStyle : Entry :=
  { term := .prm 99,
    doms := [(false, .prm 0), (true, cls [.prm 0])],
    cod := cls [.prm 0] }

private def base : Entry :=
  { term := .nd 2 [], doms := [], cod := cls [.nd 1 []] }

example :
    (resolve 8 m0 (cls [.nd 1 []]) [base, eqStyle]).map Prod.fst
      = some (.nd 2 []) := by
  rfl

/-- No matching entry: resolution fails. -/
private def otherCod : Entry :=
  { term := .prm 7, doms := [], cod := cls [.nd 9 []] }

example : resolve 8 m0 (cls [.nd 1 []]) [otherCod] = none := by
  rfl

/-- A later (head) entry shadows an earlier one. -/
private def late : Entry :=
  { term := .nd 5 [], doms := [], cod := cls [.nd 1 []] }

example :
    (resolve 8 m0 (cls [.nd 1 []]) [late, otherCod]).map Prod.fst
      = some (.nd 5 []) := by
  rfl
