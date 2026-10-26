//! Syntax of the language.
//!
//! Three layers are represented here:
//!
//! * [`Raw`] -- surface syntax, as produced by the parser (still has names and
//!   source positions);
//! * [`Tm`] -- core syntax, de Bruijn indices, produced by the elaborator;
//! * [`Val`] -- semantic values, produced by the evaluator (normalization by
//!   evaluation).
//!
//! This is a direct port of `Syntax.hs` from the original project.

use std::collections::BTreeMap;
use std::rc::Rc;

pub type Name = String;

/// A metavariable (hole) identifier; `?{m}` in printed terms.
pub type MetaId = usize;

/// De Bruijn index: counts binders from a variable outwards.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Ix(pub usize);

/// De Bruijn level: counts binders from the outside in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Lvl(pub usize);

impl Lvl {
    pub const ZERO: Lvl = Lvl(0);
    pub const ONE: Lvl = Lvl(1);

    #[inline]
    pub fn inc(self) -> Lvl {
        Lvl(self.0 + 1)
    }

    #[inline]
    pub fn plus(self, n: usize) -> Lvl {
        Lvl(self.0 + n)
    }

    /// Predecessor; levels are never decremented below zero in well-formed runs.
    #[inline]
    pub fn pred(self) -> Lvl {
        Lvl(self.0.checked_sub(1).expect("level underflow"))
    }
}

/// A source position.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pos {
    pub file: Rc<str>,
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(file: impl Into<Rc<str>>, line: usize, col: usize) -> Pos {
        Pos {
            file: file.into(),
            line,
            col,
        }
    }

    /// Position of the first character of a source file.
    pub fn initial(file: impl Into<Rc<str>>) -> Pos {
        Pos::new(file, 1, 1)
    }
}

// ---------------------------------------------------------------------------
// Surface syntax
// ---------------------------------------------------------------------------

/// Patterns (only variables, `refl` and the absurd pattern `(!)`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Pn {
    Var(Name),
    Refl,
    Abs,
}

/// Surface syntax.
#[derive(Clone, PartialEq, Debug)]
pub enum Raw {
    Var(Name),
    /// `t u`
    App(Box<Raw>, Box<Raw>),
    /// `U` or `U1`, `U2`, ... -- a universe of the given level.
    U(u32),
    /// `let x : A = t; u`
    Let(Name, Box<Raw>, Box<Raw>, Box<Raw>),
    /// `(x : A) -> B`
    Pi(Name, Box<Raw>, Box<Raw>),
    /// `\ p1 p2 ... . t`
    Lam(Vec<Pn>, Box<Raw>),
    /// `refl`
    Refl,
    /// `Id A u v`
    Id(Box<Raw>, Box<Raw>, Box<Raw>),
    /// `data D params : ty where c1 : t1; ... ; body`
    Data(
        Name,
        Vec<(Name, Raw)>,
        Box<Raw>,
        Vec<(Name, Raw)>,
        Box<Raw>,
    ),
    /// `struct S params : ty where f1 : t1; ... ; body` -- sugar for a
    /// single-constructor `data` plus one projection function per field.
    Struct(
        Name,
        Vec<(Name, Raw)>,
        Box<Raw>,
        Vec<(Name, Raw)>,
        Box<Raw>,
    ),
    /// `x.f` -- field projection, sugar for the eliminator of the field's
    /// single-constructor type.
    Proj(Box<Raw>, Name),
    /// `elim D` -- the eliminator (recursor) generated for a data type.
    Elim(Box<Raw>),
    /// `_` in expression position: a hole, elaborated to a fresh
    /// metavariable (a `_` in binder or pattern position is a wildcard
    /// instead, and never becomes a `Hole`).
    Hole,
    /// `\ (!)` -- only ever built by the parser for an absurd lambda.
    Bot,
    /// Source position attached for error reporting.
    SrcPos(Pos, Box<Raw>),
}

impl Raw {
    /// Remove all source position annotations (mirrors Syntax.hs's
    /// stripPos; used by the parser tests).
    pub fn strip_pos(&self) -> Raw {
        match self {
            Raw::SrcPos(_, t) => t.strip_pos(),
            Raw::Var(x) => Raw::Var(x.clone()),
            Raw::App(t, u) => Raw::App(Box::new(t.strip_pos()), Box::new(u.strip_pos())),
            Raw::U(l) => Raw::U(*l),
            Raw::Let(x, a, t, u) => Raw::Let(
                x.clone(),
                Box::new(a.strip_pos()),
                Box::new(t.strip_pos()),
                Box::new(u.strip_pos()),
            ),
            Raw::Pi(x, a, b) => {
                Raw::Pi(x.clone(), Box::new(a.strip_pos()), Box::new(b.strip_pos()))
            }
            Raw::Lam(ps, t) => Raw::Lam(ps.clone(), Box::new(t.strip_pos())),
            Raw::Refl => Raw::Refl,
            Raw::Id(a, x, y) => Raw::Id(
                Box::new(a.strip_pos()),
                Box::new(x.strip_pos()),
                Box::new(y.strip_pos()),
            ),
            Raw::Data(d, ps, ty, cs, body) => Raw::Data(
                d.clone(),
                ps.iter()
                    .map(|(x, a)| (x.clone(), a.strip_pos()))
                    .collect(),
                Box::new(ty.strip_pos()),
                cs.iter()
                    .map(|(x, t)| (x.clone(), t.strip_pos()))
                    .collect(),
                Box::new(body.strip_pos()),
            ),
            Raw::Elim(a) => Raw::Elim(Box::new(a.strip_pos())),
            Raw::Struct(d, ps, ty, fs, body) => Raw::Struct(
                d.clone(),
                ps.iter()
                    .map(|(x, a)| (x.clone(), a.strip_pos()))
                    .collect(),
                Box::new(ty.strip_pos()),
                fs.iter()
                    .map(|(x, t)| (x.clone(), t.strip_pos()))
                    .collect(),
                Box::new(body.strip_pos()),
            ),
            Raw::Proj(x, f) => Raw::Proj(Box::new(x.strip_pos()), f.clone()),
            Raw::Hole => Raw::Hole,
            Raw::Bot => Raw::Bot,
        }
    }
}

// ---------------------------------------------------------------------------
// Inductive declarations
// ---------------------------------------------------------------------------

/// One constructor of an inductive type.
#[derive(Clone, Debug)]
pub struct CtorInfo {
    pub name: Name,
    /// The constructor's public type `Pi(params) Pi(args) D params`, stored as
    /// a term of the declaration's context extended with the datatype itself
    /// as the innermost binder: the datatype is therefore the *single free
    /// variable* of `ty` (a `Var` whose index equals the number of binders
    /// crossed since the root), which keeps [`IndDef`] acyclic.  The
    /// elaborator closes it with [`crate::elab::close_data`] at the use sites.
    pub ty: Tm,
    /// Number of arguments after the parameters.
    pub num_args: usize,
    /// `recursive[j]`: the j-th argument (after the parameters) has type
    /// `D params`, so the eliminator hands its method an induction hypothesis
    /// for it.
    pub recursive: Vec<bool>,
}

/// A user-declared inductive type.
#[derive(Clone, Debug)]
pub struct IndDef {
    pub name: Name,
    /// The universe the type was declared in: `D : U level` (after the
    /// parameters).  The generated eliminator uses it for its motive.
    pub level: u32,
    /// The parameter telescope, stored in the datatype's own context:
    /// `params[i].1` is valid in `[..., D, params[0..i]]`, i.e. the datatype
    /// is the single free variable (the same convention as constructor
    /// types).
    pub params: Vec<(Name, Tm)>,
    pub ctors: Vec<CtorInfo>,
}

impl IndDef {
    pub fn num_params(&self) -> usize {
        self.params.len()
    }
}

/// Inductive types are nominal: two declarations are the same type only if
/// they are the same declaration, so equality is pointer equality of the
/// definition (the derived structural `PartialEq` would conflate two
/// identical-but-distinct declarations).
impl PartialEq for IndDef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for IndDef {}

// ---------------------------------------------------------------------------
// Core syntax
// ---------------------------------------------------------------------------

pub type Ty = Tm;

/// Core syntax, with de Bruijn indices.
#[derive(Clone, PartialEq, Debug)]
pub enum Tm {
    Var(Ix),
    /// A universe; level 0 is written `U` in the surface syntax and the
    /// tower is predicative: `U0 : U1 : U2 : ...`.
    U(u32),
    Let(Name, Box<Ty>, Box<Tm>, Box<Tm>),
    Pi(Name, Box<Ty>, Box<Ty>),
    Lam(Vec<Pn>, Box<Tm>),
    App(Box<Tm>, Box<Tm>),
    Id(Box<Ty>, Box<Tm>, Box<Tm>),
    Refl,
    /// A data type former (`D` itself, awaiting its parameters).
    Data(Rc<IndDef>),
    /// Constructor `i` of a data type (awaiting parameters and arguments).
    Ctor(Rc<IndDef>, usize),
    /// The eliminator of a data type.
    Elim(Rc<IndDef>),
    /// A metavariable (hole), awaiting its solution.  Its type and solution
    /// live in [`crate::meta`]'s metacontext; evaluation reads through solved
    /// metas on demand.
    Meta(MetaId),
    Bot,
}

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

/// A spine of arguments: `Sp | s :> v`. Represented as a plain vector.
pub type Spine = Vec<Val>;

/// An evaluation environment, indexed by de Bruijn index from the right.
#[derive(Clone, PartialEq, Debug)]
pub struct Env(pub Spine);

impl Env {
    pub fn empty() -> Env {
        Env(Vec::new())
    }

    /// Extend the environment with more values (`vs +++ s`).
    pub fn extend(&self, more: &Spine) -> Env {
        let mut vs = self.0.clone();
        vs.extend_from_slice(more);
        Env(vs)
    }

    /// Push a single value.
    pub fn push(&self, v: Val) -> Env {
        let mut vs = self.0.clone();
        vs.push(v);
        Env(vs)
    }

    pub fn len(&self) -> Lvl {
        Lvl(self.0.len())
    }
}

/// A closure: an environment together with an open body.
///
/// `Closure Γ n = Env Γ Δ × Tm (Δ + n)`, i.e. a function expecting `n` more
/// values.  Substitutions act on the environment only, which is why closures
/// are stable under substitution (see [`crate::eval::subst_cl`]).
#[derive(Clone, PartialEq, Debug)]
pub struct Closure {
    pub env: Env,
    pub body: Rc<Tm>,
}

impl Closure {
    pub fn new(env: Env, body: Tm) -> Closure {
        Closure {
            env,
            body: Rc::new(body),
        }
    }

    /// `cl $$ v`
    #[inline]
    pub fn apply(&self, v: Val) -> Val {
        crate::eval::capp(self, &vec![v])
    }
}

/// One layer of an explicit substitution: a finite map, level-indexed.
#[derive(Clone, PartialEq, Debug)]
pub struct Layer {
    pub dom: Lvl,
    pub cod: Lvl,
    pub subs: BTreeMap<usize, Val>,
}

/// An explicit substitution, level-indexed.
///
/// `Sub d c s` maps variables of the context of size `c` to values living in
/// the context of size `d`; variables not mapped are mapped to themselves.
///
/// A substitution is a *sequence of layers*, newest first, rather than a single
/// map: composing `s2` after `s1` concatenates the layer lists and leaves the
/// values alone (see `eval::subst_sub`).  The original composes eagerly with
/// `IM.map`, which relies on laziness to stay cheap; in a strict language that
/// recursion turns into an exponential blow-up (`subst_sub` was called 2^n
/// times for an n-step transitivity proof).  Lookup resolves the layers on
/// demand instead.
#[derive(Clone, PartialEq, Debug)]
pub struct Sub {
    pub dom: Lvl,
    pub cod: Lvl,
    pub layers: Vec<Rc<Layer>>,
}

impl Sub {
    /// A single-layer substitution.
    pub fn new(dom: Lvl, cod: Lvl, subs: BTreeMap<usize, Val>) -> Sub {
        Sub {
            dom,
            cod,
            layers: vec![Rc::new(Layer { dom, cod, subs })],
        }
    }

    /// The identity substitution on `n` variables.
    pub fn id(n: Lvl) -> Sub {
        Sub {
            dom: n,
            cod: n,
            layers: Vec::new(),
        }
    }

    /// The empty substitution `Sub d .`.
    pub fn empty(d: Lvl) -> Sub {
        Sub {
            dom: d,
            cod: Lvl::ZERO,
            layers: Vec::new(),
        }
    }
}

/// Semantic values.
///
/// Every recursive field is an [`Rc`], which mirrors the sharing that laziness
/// provides in the Haskell original.
#[derive(Clone, PartialEq, Debug)]
pub enum Val {
    /// Neutral value: a stuck computation (variable, unmatched lambda, ...).
    Ne(Rc<Ne>),
    /// A delayed substitution, eliminated by forcing.
    Sub(Rc<Val>, Rc<Sub>),
    /// A universe; see [`Tm::U(0)`].
    U(u32),
    Pi(Name, Rc<Val>, Rc<Closure>),
    Id(Rc<Val>, Rc<Val>, Rc<Val>),
    Refl,
    /// The value of `Tm::Bot`, i.e. the (unreachable) right hand side of an
    /// absurd pattern.  The original has no value for `Bot` and crashes if an
    /// absurd-pattern lambda is ever forced (see NOTES.md deviation 7).
    Bot,
}

impl Val {
    #[inline]
    pub fn ne(n: Ne) -> Val {
        Val::Ne(Rc::new(n))
    }

    /// `VSub v s`
    #[inline]
    pub fn vsub(v: Val, s: Sub) -> Val {
        Val::Sub(Rc::new(v), Rc::new(s))
    }

    /// The free variable at level `l`.
    #[inline]
    pub fn vvar(l: Lvl) -> Val {
        Val::ne(Ne::Var(l, Vec::new()))
    }

    #[inline]
    pub fn pi(x: impl Into<Name>, a: Val, b: Closure) -> Val {
        Val::Pi(x.into(), Rc::new(a), Rc::new(b))
    }

    #[inline]
    pub fn id(a: Val, x: Val, y: Val) -> Val {
        Val::Id(Rc::new(a), Rc::new(x), Rc::new(y))
    }

    /// A data type former applied to no parameters yet.
    #[inline]
    pub fn data(def: &Rc<IndDef>) -> Val {
        Val::ne(Ne::Data(def.clone(), Vec::new()))
    }

    /// Constructor `i` of `def`, applied to no arguments yet.
    #[inline]
    pub fn ctor(def: &Rc<IndDef>, i: usize) -> Val {
        Val::ne(Ne::Ctor(def.clone(), i, Vec::new()))
    }

    /// The eliminator of `def`, awaiting its spine.
    #[inline]
    pub fn elim(def: &Rc<IndDef>) -> Val {
        Val::ne(Ne::Elim(def.clone(), Vec::new()))
    }
}

/// Neutral terms: computations that are stuck.
#[derive(Clone, PartialEq, Debug)]
pub enum Ne {
    /// A delayed substitution on a neutral term.
    Sub(Rc<Ne>, Rc<Sub>),
    /// A variable at a level, applied to a spine of arguments.
    Var(Lvl, Spine),
    /// A lambda that cannot reduce yet, applied to a (too short) spine.
    /// Lambda is *not* stable under substitution, hence the extra spine.
    Lam(Vec<Pn>, Lvl, Closure, Spine),
    /// A data type former applied to (at most) its parameters.  Rigid: it
    /// never unstuck, it only accumulates parameters.
    Data(Rc<IndDef>, Spine),
    /// A constructor applied to parameters and arguments (possibly only some
    /// of them).  Rigid as well; the eliminator is what consumes it.
    Ctor(Rc<IndDef>, usize, Spine),
    /// An eliminator applied to `[params, motive, methods..., target]`.  It
    /// reduces once the spine is complete *and* the target is a saturated
    /// constructor; otherwise it stays stuck (see [`crate::eval::elim_step`]).
    Elim(Rc<IndDef>, Spine),
    /// A metavariable applied to a (possibly partial) spine.  *Flexible*: it
    /// unstucks as soon as the meta is solved, which is why it must never be
    /// treated as rigid by [`crate::eval::frc`].
    Meta(MetaId, Spine),
}

/// Result of a reduction step: a value plus whether progress was made.
///
/// The `progressed` flag matters: forcing a *blocked* neutral must not try to
/// reduce it again (that would loop), see [`crate::eval::frc_res`].
#[derive(Clone, Debug)]
pub struct Res {
    pub val: Val,
    pub progressed: bool,
}

#[inline]
pub fn block(v: Val) -> Res {
    Res {
        val: v,
        progressed: false,
    }
}

#[inline]
pub fn progress(v: Val) -> Res {
    Res {
        val: v,
        progressed: true,
    }
}

/// Number of patterns that actually bind a variable.
pub fn num_pvars(ps: &[Pn]) -> Lvl {
    Lvl(ps.iter().filter(|p| matches!(p, Pn::Var(_))).count())
}

/// The level corresponding to a de Bruijn index in a context of size `l`.
#[inline]
pub fn lvl2ix(l: Lvl, x: Lvl) -> Ix {
    Ix(l.0
        .checked_sub(x.0 + 1)
        .expect("lvl2ix: variable level is out of scope"))
}

/// Look up de Bruijn index `i`, counting from the right end of the spine.
pub fn lookup_env(i: Ix, env: &Env) -> Val {
    let vs = &env.0;
    let n = vs.len();
    if i.0 >= n {
        panic!("lookup_env: index out of bounds");
    }
    vs[n - 1 - i.0].clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lvl2ix_counts_from_the_right() {
        // In a context of size 3, the newest variable (level 2) has index 0.
        assert_eq!(lvl2ix(Lvl(3), Lvl(2)), Ix(0));
        assert_eq!(lvl2ix(Lvl(3), Lvl(1)), Ix(1));
        assert_eq!(lvl2ix(Lvl(3), Lvl(0)), Ix(2));
    }

    #[test]
    fn env_is_indexed_from_the_right() {
        let env = Env(vec![Val::Refl, Val::Bot, Val::U(0)]);
        assert_eq!(lookup_env(Ix(0), &env), Val::U(0));
        assert_eq!(lookup_env(Ix(1), &env), Val::Bot);
        assert_eq!(lookup_env(Ix(2), &env), Val::Refl);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn env_lookup_out_of_bounds_panics() {
        let _ = lookup_env(Ix(1), &Env(vec![Val::Refl]));
    }

    #[test]
    fn extend_and_push_preserve_the_right_most_binding() {
        let env = Env(vec![Val::Refl]);
        assert_eq!(env.push(Val::U(0)).0, vec![Val::Refl, Val::U(0)]);
        assert_eq!(
            env.extend(&vec![Val::U(0), Val::Bot]).0,
            vec![Val::Refl, Val::U(0), Val::Bot]
        );
        assert_eq!(env.len(), Lvl(1));
    }

    #[test]
    fn num_pvars_counts_only_variable_patterns() {
        use Pn::*;
        assert_eq!(num_pvars(&[]), Lvl(0));
        assert_eq!(num_pvars(&[Var("x".into()), Refl, Abs]), Lvl(1));
        assert_eq!(num_pvars(&[Refl, Refl]), Lvl(0));
    }

    #[test]
    fn lvl_pred_and_inc() {
        assert_eq!(Lvl(0).inc(), Lvl(1));
        assert_eq!(Lvl(3).pred(), Lvl(2));
        assert_eq!(Lvl(2).plus(3), Lvl(5));
    }
}
