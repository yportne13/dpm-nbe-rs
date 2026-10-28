/-!
# `let rec`: the structural termination check (NOTES.md deviation 19)

Formalization of the checker at the *scope* level, in the style of the
companion files.  The model mirrors the implementation (`elab.rs`'s
`calls_decrease` / `dec_go` / `dec_method`): terms carry the shapes the
walk inspects — variables, application spines, single-binder lambdas, the
fixpoint's self-reference (`fix`), an opaque constructor head (`ctor`),
and an eliminator head with each method's recursive-argument flags
(`elim fs`).  Subterm depths live in a de Bruijn-indexed environment, and
`dec` is the walk: every `fix`-headed spine must be saturated past `pos`
with a depth-≥ 1 variable there; an eliminator's methods are walked with
the target's depth.

The classical structural-recursion argument is carried one link per
theorem:

* `methodEntries_some` — a method's recursive-argument binders sit at
  depth `d + 1` (≥ 1 whenever the target had a depth): what the walk's
  *acceptance* of a call means.
* `ctorArg_smaller` — a constructor argument is strictly smaller than the
  constructor application (`size` counts constructor nodes).
* `ctorArgOf_size` / `ctorArgTrans_size` / `ctorArgTrans_irrefl` — the
  "is a constructor argument of" relation strictly decreases size and is
  transitively irreflexive: the well-founded chain the checker routes
  every self-call through.
* The sanity theorems mirror the probe matrix: the walk rejects a call on
  the argument itself (depth 0) and on a constructed term, accepts a call
  on a method binder, and rejects a method-binder call on the eliminator's
  target variable.

The one link left as prose (the same oracle boundary the companion files
draw at the β-law) is the eliminator's defining equation — that a method's
recursive-argument binders are *instantiated* with the constructor's
arguments.  With it, the composition is: accepted call ⟹ argument is a
recursive method binder at depth ≥ 1 ⟹ instantiated with a constructor
argument (elim equation) ⟹ strictly smaller (`ctorArg_smaller`) ⟹
well-founded (`ctorArgTrans_irrefl`) — so unfolding terminates.

The `Nat` fuel parameter of the walk is a totality artifact of this
formalization: at exhaustion the walk rejects (conservative), and every
statement below uses explicitly ample fuel.  Machine-checked with Lean
4.35.0 (no mathlib): `lean Termination.lean`.
-/

universe u

/-- Terms over `n` de Bruijn indices, with the shapes the termination
    walk inspects.  (`fix` is the fixpoint's self-reference; `ctor` and
    `elim fs` are opaque heads carrying, for the eliminator, each
    method's recursive-argument flags.) -/
inductive Tm : Nat → Type where
  | var : Fin n → Tm n
  | app : Tm n → Tm n → Tm n
  | lam : Tm (n + 1) → Tm n
  | fix : Tm n
  | ctor : Tm n
  | elim : List (List Bool) → Tm n
  | mvar : Nat → Tm n
  deriving DecidableEq

/-- Subterm depths by de Bruijn index (index 0 = innermost binder):
    `some d` marks a variable that is a d-times-nested constructor
    argument of the fixpoint's designated argument. -/
abbrev DepthEnv := List (Option Nat)

/-- Peel an application spine into head and arguments. -/
def spineOf : Tm n → Tm n × List (Tm n)
  | .app f a =>
    let (h, args) := spineOf f
    (h, args ++ [a])
  | t => (t, [])

/-- Whether the term is a variable of depth at least one. -/
def isVarDepthGe : Tm n → DepthEnv → Bool
  | .var v, env =>
    match env.getD v.val none with
    | some d => 1 <= d
    | none => false
  | _, _ => false

/-- The depths a method's telescope assigns: per flag, the argument binder
    (`td.map (+1)` when recursive, `none` otherwise) and, after a recursive
    one, the induction hypothesis (`none`).  Outermost binder first, the
    order the walk consumes them in. -/
def methodEntries : List Bool → Option Nat → List (Option Nat)
  | [], _ => []
  | true :: rs, td => td.map (· + 1) :: none :: methodEntries rs td
  | false :: rs, td => none :: methodEntries rs td

mutual

/-- Walk a method body under its telescope, consuming depths in order
    (outermost binder first, each pushed at the environment's front).
    A shape mismatch — depths remain but the term is no lambda — is
    conservative: the unconsumed binders are absent from the environment,
    so any call through them looks depth-less and rejects. -/
def decMethod : Nat → Tm n → List (Option Nat) → DepthEnv → Nat → Bool
  | 0, _, _, _, _ => false
  | f + 1, .lam b, e :: es, env, pos => decMethod f b es (e :: env) pos
  | f + 1, t, _ :: _, env, pos => dec f t env pos
  | f, t, [], env, pos => dec f t env pos

/-- The walk itself. -/
def dec : Nat → Tm n → DepthEnv → Nat → Bool
  | 0, _, _, _ => false
  | _ + 1, .var _, _, _ => true
  | f + 1, .lam b, env, pos => dec f b (none :: env) pos
  | _ + 1, .fix, _, _ => false -- a bare self-reference is never saturated
  | _ + 1, .ctor, _, _ => true
  | _ + 1, .elim _, _, _ => true
  | _ + 1, .mvar _, _, _ => true
  | f + 1, t@(.app ..), env, pos =>
    let (h, args) := spineOf t
    match h with
    | .fix =>
      pos < args.length
        && isVarDepthGe (args.getD pos (.ctor)) env
        && args.all (fun a => dec f a env pos)
    | .elim fs => decElim f fs args env pos
    | _ => dec f h env pos && args.all (fun a => dec f a env pos)

/-- An eliminator spine `[motive, m₀..mₖ, target, extra..]`: the target
    seeds the methods' depths; each method is walked per its flags. -/
def decElim : Nat → List (List Bool) → List (Tm n) → DepthEnv → Nat → Bool
  | 0, _, _, _, _ => false
  | f + 1, fs, args, env, pos =>
    let n := fs.length
    let td :=
      match args.getD (n + 1) (.ctor) with
      | .var v => env.getD v.val none
      | _ => none
    decMethods f fs args 1 td env pos
      && dec f (args.getD 0 (.ctor)) env pos
      && (args.drop (n + 1)).all (fun a => dec f a env pos)

def decMethods : Nat → List (List Bool) → List (Tm n) → Nat →
    Option Nat → DepthEnv → Nat → Bool
  | 0, _, _, _, _, _, _ => false
  | _ + 1, [], _, _, _, _, _ => true
  | f + 1, fl :: rest, args, i, td, env, pos =>
    decMethod f (args.getD i (.ctor)) (methodEntries fl td) env pos
      && decMethods f rest args (i + 1) td env pos

end

/-- Walk the fixpoint body's own λ-telescope: the binder at position
    `pos` (the designated recursive argument) gets depth 0, the others
    none; then the walk proper. -/
def decBody : Nat → Tm n → Nat → Nat → DepthEnv → Bool
  | 0, _, _, _, _ => false
  | f + 1, .lam b, i, pos, env =>
    decBody f b (i + 1) pos ((if i = pos then some 0 else none) :: env)
  | f + 1, t, _, pos, env => dec f t env pos

/-! ## Method depths -/

/-- **Method depths.**  At target depth `d`, every depth a method's
    telescope assigns is either `none` (an induction hypothesis or a
    non-recursive argument) or exactly `some (d + 1)` — so an accepted
    call's argument binder, whenever it has a depth, is a *strict* subterm
    position. -/
theorem methodEntries_some {flags : List Bool} {d : Nat} :
    ∀ e ∈ methodEntries flags (some d), e = some (d + 1) ∨ e = none := by
  induction flags with
  | nil => intro e h; cases h
  | cons f rest ih =>
    intro e h
    cases f with
    | true =>
      simp only [methodEntries, List.mem_cons] at h
      rcases h with h | h | h
      · rw [h]; exact Or.inl rfl
      · exact Or.inr h
      · exact ih e h
    | false =>
      simp only [methodEntries, List.mem_cons] at h
      rcases h with h | h
      · exact Or.inr h
      · exact ih e h

/-! ## Constructor sizes -/

/-- Constructor nodes of a term (the measure). -/
def size : Tm n → Nat
  | .var _ => 0
  | .app f a => size f + size a
  | .lam b => size b
  | .fix => 0
  | .ctor => 1
  | .elim _ => 0
  | .mvar _ => 0

/-- The constructor-node total of a list of terms. -/
def sumSize (l : List (Tm n)) : Nat := (l.map size).sum

/-- The left-nested constructor application `c a₀ a₁ .. aₖ`. -/
def ctorSpine (l : List (Tm n)) : Tm n :=
  l.foldl (fun acc x => Tm.app acc x) Tm.ctor

theorem sumSize_cons (x : Tm n) (l : List (Tm n)) :
    sumSize (x :: l) = size x + sumSize l := rfl

theorem size_foldl (acc : Tm n) (l : List (Tm n)) :
    size (l.foldl (fun a x => Tm.app a x) acc) = size acc + sumSize l := by
  induction l generalizing acc with
  | nil => rfl
  | cons x rest ih =>
    rw [List.foldl_cons, ih, sumSize_cons]
    show size acc + size x + sumSize rest = size acc + (size x + sumSize rest)
    exact Nat.add_assoc _ _ _

theorem size_ctorSpine (l : List (Tm n)) :
    size (ctorSpine l) = 1 + sumSize l := by
  rw [ctorSpine, size_foldl]
  rfl

theorem le_sumSize_of_mem {l : List (Tm n)} {a : Tm n} (h : a ∈ l) :
    size a ≤ sumSize l := by
  induction l with
  | nil => cases h
  | cons x rest ih =>
    rcases List.mem_cons.mp h with h' | h'
    · rw [h', sumSize_cons]; exact Nat.le_add_right _ _
    · rw [sumSize_cons]; exact Nat.le_trans (ih h') (Nat.le_add_left _ _)

/-- **Constructor arguments are strictly smaller.** -/
theorem ctorArg_smaller {args : List (Tm n)} {a : Tm n} (h : a ∈ args) :
    size a < size (ctorSpine args) := by
  have hle := le_sumSize_of_mem h
  rw [size_ctorSpine]
  omega

/-! ## The subterm relation is well-founded -/

/-- The spine equation for applications. -/
theorem spineOf_cons (f a : Tm n) :
    spineOf (Tm.app f a) = ((spineOf f).1, (spineOf f).2 ++ [a]) := rfl

/-- A spine with a constructor head is the constructor application of
    its arguments. -/
theorem spineOf_ctor_eq {t : Tm n} :
    (spineOf t).1 = Tm.ctor → t = ctorSpine (spineOf t).2 := by
  induction t with
  | var v => intro h; simp only [spineOf] at h; exact absurd h (by simp)
  | lam b => intro h; simp only [spineOf] at h; exact absurd h (by simp)
  | fix => intro h; simp only [spineOf] at h; exact absurd h (by simp)
  | ctor => intro _; rfl
  | elim fs => intro h; simp only [spineOf] at h; exact absurd h (by simp)
  | mvar g => intro h; simp only [spineOf] at h; exact absurd h (by simp)
  | app f a ih =>
    intro h
    have h' : (spineOf f).1 = Tm.ctor := h
    have e1 : Tm.app f a = Tm.app (ctorSpine (spineOf f).2) a :=
      congrArg (fun g => Tm.app g a) (ih h')
    have e2 : ctorSpine (spineOf (Tm.app f a)).2
        = ctorSpine ((spineOf f).2 ++ [a]) := rfl
    rw [e2, e1, ctorSpine, ctorSpine, List.foldl_append, List.foldl_cons,
        List.foldl_nil]

/-- `a` is a constructor argument of `t`. -/
inductive CtorArgOf : Tm n → Tm n → Prop
  | mk {t : Tm n} (hc : (spineOf t).1 = Tm.ctor) {a : Tm n}
      (h : a ∈ (spineOf t).2) : CtorArgOf a t

theorem ctorArgOf_size {a t : Tm n} (h : CtorArgOf a t) : size a < size t := by
  rcases h with ⟨hc, hmem⟩
  rw [spineOf_ctor_eq hc]
  exact ctorArg_smaller hmem

/-- The transitive closure of `CtorArgOf`. -/
inductive CtorArgTrans : Tm n → Tm n → Prop
  | single {a b : Tm n} (h : CtorArgOf a b) : CtorArgTrans a b
  | step {a b c : Tm n} (h : CtorArgOf a b) (r : CtorArgTrans b c) :
      CtorArgTrans a c

theorem ctorArgTrans_size {a b : Tm n} (h : CtorArgTrans a b) :
    size a < size b := by
  induction h with
  | single h => exact ctorArgOf_size h
  | step h r ih => exact Nat.lt_trans (ctorArgOf_size h) ih

/-- **Well-foundedness**: nothing is a transitive constructor-argument of
    itself, so every chain of strict subterms is finite (sizes strictly
    decrease along it). -/
theorem ctorArgTrans_irrefl {t : Tm n} (h : CtorArgTrans t t) : False :=
  Nat.lt_irrefl (size t) (ctorArgTrans_size h)

/-! ## Sanity: the probe matrix, in the model -/

/-- A call on the argument itself (depth 0) is rejected. -/
example :
    dec 16 (Tm.app Tm.fix (Tm.var (⟨0, by omega⟩ : Fin 1))) [some 0] 0
        = false := by
  native_decide

/-- A call on a constructed term is rejected. -/
example :
    dec 16 (Tm.app Tm.fix (Tm.app Tm.ctor (Tm.var (⟨0, by omega⟩ : Fin 2))))
        [some 0] 0 = false := by
  native_decide

/-- A call on a method binder (depth 1, over an induction hypothesis) is
    accepted. -/
example :
    dec 16 (Tm.app Tm.fix (Tm.var (⟨1, by omega⟩ : Fin 3)))
        [none, some 1, some 0] 0 = true := by
  native_decide

/-- The full shape: `λ n . elim [[true]] m (λ m r . fix m) n`,
    parameterised by the variable the method's self-call takes. -/
private def elimBody (call : Fin 3) : Tm 1 :=
  List.foldl Tm.app (Tm.elim [[true]])
    [Tm.mvar 0,
     Tm.lam (Tm.lam (Tm.app Tm.fix (Tm.var call))),
     Tm.var (⟨0, by omega⟩ : Fin 1)]

/-- The method's recursive binder sits at depth 1, so the call is
    accepted. -/
example :
    decBody 32 (Tm.lam (elimBody (⟨1, by omega⟩ : Fin 3)))
        0 0 [] = true := by
  native_decide

/-- The same shape calling the eliminator's *target* (the fixpoint's own
    argument, depth 0) is rejected. -/
example :
    decBody 32 (Tm.lam (elimBody (⟨2, by omega⟩ : Fin 3)))
        0 0 [] = false := by
  native_decide
