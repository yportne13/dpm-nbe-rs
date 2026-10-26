//! Pattern unification for metavariables, with pruning.
//!
//! Port of `Unification.hs` from elaboration-zoo's `05-pruning`, minus
//! implicit arguments (dpm-nbe is explicit-only, so a [`Pruning`] is a plain
//! `Vec<bool>`).  Solving `?m spine =? rhs` inverts the spine into a partial
//! renaming and renames `rhs` under it; renaming may *prune* metas that the
//! rhs reaches only through variables the spine does not provide, by solving
//! them as projections onto a fresh meta with fewer arguments (which is only
//! well-typed when the pruned type still is, hence metas carry types).
//!
//! Two conventions worth stating because the zoo's code reads opposite to
//! its comments: [`PRen::ren`] maps a *use-site* variable level to its
//! *meta-context* position, the renamed right-hand side lives in the meta's
//! context (size `dom`), and lifting numbers the use-site binder with `cod`
//! -- the two context sizes stay in lockstep because they start out as the
//! two sizes being related and grow together.  A [`Pruning`] is ordered
//! outermost-binder first, `true` meaning "keep the argument".

use crate::eval::{capp, conv, conv_le, eval, frc, napp, new_vars};
use crate::meta::{self, MetaEntry};
use crate::syntax::*;
use std::collections::BTreeMap;

/// A spine mask: outermost binder first, `true` keeps the argument.
pub type Pruning = Vec<bool>;

/// A partial renaming from the use site `Γ` (`cod` variables) to the meta's
/// context `Δ` (`dom` variables): `ren` maps a Γ-variable level to its
/// Δ-position.  `occ` names the meta being solved, for the occurs check.
#[derive(Clone)]
struct PRen {
    occ: Option<MetaId>,
    dom: Lvl,
    cod: Lvl,
    ren: BTreeMap<usize, Lvl>,
}

impl PRen {
    /// Go under a binder on both sides.  The use-site binder's level is the
    /// old `cod`, exactly the level a closure opened at `cod` produces.
    fn lift(&self) -> PRen {
        let mut ren = self.ren.clone();
        ren.insert(self.cod.0, self.dom);
        PRen {
            occ: self.occ,
            dom: self.dom.inc(),
            cod: self.cod.inc(),
            ren,
        }
    }

    /// Go under a binder on the use-site side only (a pruned argument).
    fn skip(&self) -> PRen {
        PRen {
            occ: self.occ,
            dom: self.dom,
            cod: self.cod.inc(),
            ren: self.ren.clone(),
        }
    }
}

/// `invert γ sp`: read the spine as a substitution from the meta's context
/// into the use site `γ`, returning the renaming back plus, when the spine
/// is non-linear, the pruning of the duplicated arguments (which the caller
/// must be able to prune the meta's type by).  `None` when a spine entry is
/// not a variable: the meta cannot be inverted.
fn invert(gamma: Lvl, sp: &Spine) -> Option<(PRen, Option<Pruning>)> {
    let mut ren: BTreeMap<usize, Lvl> = BTreeMap::new();
    let mut nlvars: Vec<usize> = Vec::new();
    for (j, v) in sp.iter().enumerate() {
        let x = match &frc(v) {
            Val::Ne(n) => match &**n {
                Ne::Var(x, sp) if sp.is_empty() => x.0,
                _ => return None,
            },
            _ => return None,
        };
        if ren.contains_key(&x) || nlvars.contains(&x) {
            ren.remove(&x);
            if !nlvars.contains(&x) {
                nlvars.push(x);
            }
        } else {
            ren.insert(x, Lvl(j));
        }
    }
    let pruning = if nlvars.is_empty() {
        None
    } else {
        Some(
            sp.iter()
                .map(|v| match &frc(v) {
                    Val::Ne(n) => match &**n {
                        Ne::Var(x, sp) if sp.is_empty() => !nlvars.contains(&x.0),
                        _ => false,
                    },
                    _ => false,
                })
                .collect(),
        )
    };
    Some((
        PRen {
            occ: None,
            dom: Lvl(sp.len()),
            cod: gamma,
            ren,
        },
        pruning,
    ))
}

/// Remove some arguments from a closed iterated-Pi type.  Walking the chain
/// lifts over kept binders and skips dropped ones; renaming the final
/// codomain under the result both shifts the kept binders' indices and
/// rejects the pruning when a dropped binder is still referenced (a scope
/// error in the renaming).  `None` likewise when the chain has fewer binders
/// than the pruning has entries.
fn prune_ty(pr: &[bool], ty: &Tm) -> Option<Tm> {
    // A `Let` (a define from the meta's context chain) binds no argument
    // slot; keep it and keep walking.
    if let Tm::Let(x, a, v, b) = ty {
        return Some(Tm::Let(
            x.clone(),
            a.clone(),
            v.clone(),
            Box::new(prune_ty(pr, b)?),
        ));
    }
    fn go(pr: &[bool], pren: &PRen, a: &Val) -> Option<Tm> {
        match (pr.split_first(), &frc(a)) {
            (None, a) => rename(pren, a),
            (Some((true, rest)), Val::Pi(x, dom, cl)) => Some(Tm::Pi(
                x.clone(),
                Box::new(rename(pren, dom)?),
                Box::new(go(rest, &pren.lift(), &cl.apply(Val::vvar(pren.cod)))?),
            )),
            (Some((false, rest)), Val::Pi(_, _, cl)) => {
                go(rest, &pren.skip(), &cl.apply(Val::vvar(pren.cod)))
            }
            _ => None,
        }
    }
    let v = eval(&Env::empty(), ty);
    go(
        pr,
        &PRen {
            occ: None,
            dom: Lvl::ZERO,
            cod: Lvl::ZERO,
            ren: BTreeMap::new(),
        },
        &v,
    )
}

/// Wrap `t` in `l` lambdas, taking the binder names from the closed
/// iterated-Pi type.
fn lams(l: usize, ty: &Tm, t: Tm) -> Tm {
    fn go(l: usize, ty: &Tm, i: usize, t: Tm) -> Tm {
        if i == l {
            return t;
        }
        match ty {
            Tm::Pi(x, _, b) => Tm::Lam(vec![Pn::Var(x.clone())], Box::new(go(l, b, i + 1, t))),
            // A define in the closed type (from the meta's context chain)
            // binds no slot of the solution's telescope; walk under it.
            Tm::Let(_, _, _, b) => go(l, b, i, t),
            _ => t,
        }
    }
    go(l, ty, 0, t)
}

/// Prune the arguments named by `pr` out of the unsolved meta `m`: allocate
/// `m'` with the pruned type and solve `m := λ ps. m' (kept ps)`.
fn prune_meta(pr: &Pruning, m: MetaId) -> Option<MetaId> {
    let (ty, pos) = match meta::get(m) {
        MetaEntry::Unsolved { ty, pos } => (ty, pos),
        MetaEntry::Solved { .. } => return None,
    };
    let closed = ty.close();
    let pruned = prune_ty(pr, &closed)?;
    let m2 = meta::fresh(meta::MetaTy::Closed(pruned), pos);
    // λ x0 .. x_{l-1} . m' applied to the kept binders, outermost first;
    // binder j sits at index l-1-j in the body.
    let l = pr.len();
    let mut body = Tm::Meta(m2);
    for (j, keep) in pr.iter().enumerate() {
        if *keep {
            body = Tm::App(Box::new(body), Box::new(Tm::Var(Ix(l - 1 - j))));
        }
    }
    let solution = eval(&Env::empty(), &lams(l, &closed, body));
    meta::solve(m, solution, ty);
    Some(m2)
}

/// `?m spine` under the renaming: rename the spine, pruning the meta when a
/// spine variable escapes the renaming.  Returns the renamed term.
fn prune_vflex(pren: &PRen, m: MetaId, sp: &Spine) -> Option<Tm> {
    /// Walk status: the spine stayed a renaming, stopped being one, or has
    /// out-of-scope variables only pruning can account for.
    #[derive(PartialEq, Clone, Copy)]
    enum Status {
        OkRenaming,
        OkNonRenaming,
        NeedsPruning,
    }
    let mut items: Vec<Option<Tm>> = Vec::with_capacity(sp.len());
    let mut status = Status::OkRenaming;
    for v in sp {
        let fv = frc(v);
        let is_var = matches!(
            &fv,
            Val::Ne(n) if matches!(&**n, Ne::Var(_, s) if s.is_empty())
        );
        if is_var {
            let x = match &fv {
                Val::Ne(n) => match &**n {
                    Ne::Var(x, _) => x.0,
                    _ => unreachable!("guarded by is_var"),
                },
                _ => unreachable!("guarded by is_var"),
            };
            match (pren.ren.get(&x), status) {
                (Some(x2), _) => items.push(Some(Tm::Var(lvl2ix(pren.dom, *x2)))),
                (None, Status::OkNonRenaming) => return None,
                (None, _) => {
                    items.push(None);
                    status = Status::NeedsPruning;
                }
            }
        } else {
            if status == Status::NeedsPruning {
                return None;
            }
            items.push(Some(rename(pren, &fv)?));
            status = Status::OkNonRenaming;
        }
    }
    let m2 = match status {
        Status::NeedsPruning => prune_meta(&items.iter().map(|i| i.is_some()).collect(), m)?,
        Status::OkRenaming | Status::OkNonRenaming => match meta::get(m) {
            MetaEntry::Unsolved { .. } => m,
            // frc should have unfolded a solved meta before we get here
            MetaEntry::Solved { .. } => return None,
        },
    };
    let mut acc = Tm::Meta(m2);
    for i in items {
        if let Some(u) = i {
            acc = Tm::App(Box::new(acc), Box::new(u));
        }
    }
    Some(acc)
}

/// Rename a spine onto `head`.
fn rename_sp(pren: &PRen, head: Tm, sp: &Spine) -> Option<Tm> {
    let mut acc = head;
    for v in sp {
        acc = Tm::App(Box::new(acc), Box::new(rename(pren, &frc(v))?));
    }
    Some(acc)
}

/// Rename a use-site value into a term of the meta's context.  `None` on a
/// scope error, an occurs-check failure or an unprunable meta.
fn rename(pren: &PRen, v: &Val) -> Option<Tm> {
    match &frc(v) {
        Val::Ne(n) => match &**n {
            Ne::Meta(m2, sp) => {
                if pren.occ == Some(*m2) {
                    return None; // occurs check
                }
                prune_vflex(pren, *m2, sp)
            }
            Ne::Var(x, sp) => {
                let x2 = *pren.ren.get(&x.0)?;
                rename_sp(pren, Tm::Var(lvl2ix(pren.dom, x2)), sp)
            }
            Ne::Data(d, sp) => rename_sp(pren, Tm::Data(d.clone()), sp),
            Ne::Ctor(d, i, sp) => rename_sp(pren, Tm::Ctor(d.clone(), *i), sp),
            Ne::Elim(d, sp) => rename_sp(pren, Tm::Elim(d.clone()), sp),
            Ne::Lam(ps, k, cl, sp) => {
                // A stuck (possibly partially applied) pattern lambda.  Open
                // its binders with fresh use-site variables (numbered from
                // `cod`, matching `lift`), rename the body, and re-apply to
                // the renamed spine.
                let lifted = lift_n(pren, k.0);
                let body = capp(cl, &new_vars(pren.cod, *k));
                let mut acc = Tm::Lam(ps.clone(), Box::new(rename(&lifted, &body)?));
                for u in sp {
                    acc = Tm::App(Box::new(acc), Box::new(rename(pren, &frc(u))?));
                }
                Some(acc)
            }
            Ne::Sub(..) => return None, // frc collapses substitutions first
        },
        Val::Pi(x, a, cl) => {
            let a2 = rename(pren, a)?;
            let body = rename(&pren.lift(), &cl.apply(Val::vvar(pren.cod)))?;
            Some(Tm::Pi(x.clone(), Box::new(a2), Box::new(body)))
        }
        Val::U(l) => Some(Tm::U(*l)),
        Val::Id(a, x, y) => Some(Tm::Id(
            Box::new(rename(pren, a)?),
            Box::new(rename(pren, x)?),
            Box::new(rename(pren, y)?),
        )),
        Val::Refl => Some(Tm::Refl),
        Val::Bot => Some(Tm::Bot),
        Val::Sub(..) => return None,
    }
}

/// Lift a renaming under `n` binders.
fn lift_n(pren: &PRen, n: usize) -> PRen {
    let mut p = PRen {
        occ: pren.occ,
        dom: pren.dom,
        cod: pren.cod,
        ren: pren.ren.clone(),
    };
    for _ in 0..n {
        p = p.lift();
    }
    p
}

/// Solve `γ ⊢ ?m spine =? rhs`.  `false` (with no metas solved) when the
/// problem is not a pattern unification problem or the rhs escapes the
/// spine's renaming.
pub fn solve(gamma: Lvl, m: MetaId, sp: &Spine, rhs: &Val) -> bool {
    let Some((pren, prune_nonlinear)) = invert(gamma, sp) else {
        return false;
    };
    solve_with_pren(m, &pren, prune_nonlinear.as_deref(), rhs)
}

/// Solve `?m` given the inversion of its spine.
fn solve_with_pren(m: MetaId, pren: &PRen, prune_nonlinear: Option<&[bool]>, rhs: &Val) -> bool {
    let ty = match meta::get(m) {
        MetaEntry::Unsolved { ty, .. } => ty,
        MetaEntry::Solved { .. } => return false,
    };
    let closed = ty.close();
    // A non-linear spine only solves when the duplicated arguments can be
    // pruned off the meta's type: the solution would be ill-typed otherwise.
    if let Some(pr) = prune_nonlinear {
        if prune_ty(pr, &closed).is_none() {
            return false;
        }
    }
    let rhs2 = rename(&PRen { occ: Some(m), ..pren.clone() }, rhs);
    let Some(rhs2) = rhs2 else {
        return false;
    };
    let solution = eval(&Env::empty(), &lams(pren.dom.0, &closed, rhs2));
    meta::solve(m, solution, ty);
    true
}

/// Unify two spines pointwise.
fn unify_sp(l: Lvl, sp: &Spine, sp2: &Spine) -> bool {
    sp.len() == sp2.len() && sp.iter().zip(sp2.iter()).all(|(a, b)| unify(l, a, b))
}

/// Solve `γ ⊢ ?m sp =? ?m' sp'`: invert the longer spine (usually the inner
/// scope), falling back to solving the other side when it does not invert.
fn flex_flex(gamma: Lvl, m: MetaId, sp: &Spine, m2: MetaId, sp2: &Spine) -> bool {
    let go = |ma: MetaId, spa: &Spine, mb: MetaId, spb: &Spine| -> bool {
        match invert(gamma, spa) {
            None => solve(gamma, mb, spb, &Val::ne(Ne::Meta(ma, spa.clone()))),
            Some((pren, pr)) => {
                solve_with_pren(ma, &pren, pr.as_deref(), &Val::ne(Ne::Meta(mb, spb.clone())))
            }
        }
    };
    if sp.len() < sp2.len() {
        go(m2, sp2, m, sp)
    } else {
        go(m, sp, m2, sp2)
    }
}

/// `?m sp =? ?m sp'` with both spines renamings: prune every argument on
/// which they disagree; otherwise unify pointwise.
fn intersect(l: Lvl, m: MetaId, sp: &Spine, sp2: &Spine) -> bool {
    if sp.len() != sp2.len() {
        return unify_sp(l, sp, sp2);
    }
    let mut pr: Pruning = Vec::with_capacity(sp.len());
    for (a, b) in sp.iter().zip(sp2.iter()) {
        match (&frc(a), &frc(b)) {
            (Val::Ne(n1), Val::Ne(n2)) => match (&**n1, &**n2) {
                (Ne::Var(x, s1), Ne::Var(x2, s2)) if s1.is_empty() && s2.is_empty() => {
                    pr.push(x == x2)
                }
                _ => return unify_sp(l, sp, sp2),
            },
            _ => return unify_sp(l, sp, sp2),
        }
    }
    if pr.iter().any(|b| !b) {
        prune_meta(&pr, m).is_some()
    } else {
        true
    }
}

/// Unify two values of the same type at context size `l`, solving
/// metavariables as pattern unification allows.  Failure leaves already
/// recorded solutions in place -- every caller treats failure as a hard
/// elaboration error, so no later check observes a partial solution.
pub fn unify(l: Lvl, t: &Val, u: &Val) -> bool {
    let t = frc(t);
    let u = frc(u);
    match (&t, &u) {
        (Val::U(i), Val::U(j)) => i == j,
        (Val::Pi(_, a, b), Val::Pi(_, a2, b2)) => {
            unify(l, a, a2) && unify(l.inc(), &b.apply(Val::vvar(l)), &b2.apply(Val::vvar(l)))
        }
        (Val::Id(a, x, y), Val::Id(a2, x2, y2)) => {
            unify(l, a, a2) && unify(l, x, x2) && unify(l, y, y2)
        }
        (Val::Refl, Val::Refl) => true,
        (Val::Bot, Val::Bot) => true,
        (Val::Ne(n), Val::Ne(n2)) if flex_head(n) && flex_head(n2) => match (&**n, &**n2) {
            (Ne::Meta(m, sp), Ne::Meta(m2, sp2)) => {
                if m == m2 {
                    intersect(l, *m, sp, sp2)
                } else {
                    flex_flex(l, *m, sp, *m2, sp2)
                }
            }
            _ => unreachable!("guarded by flex_head"),
        },
        // Eta: a stuck lambda unifies with anything by one argument.  Only
        // variable patterns eta-expand -- `refl`/`(!)` patterns get stuck on
        // a fresh variable and would loop forever.
        (Val::Ne(n), _) if lam_var_patterns(n) => eta_unify(l, &t, &u),
        (_, Val::Ne(n)) if lam_var_patterns(n) => eta_unify(l, &t, &u),
        // Two lambdas with non-variable patterns: conversion, no solving.
        (Val::Ne(n), Val::Ne(n2)) if lam_any(n) && lam_any(n2) => conv(l, &t, &u),
        (Val::Ne(n), _) if lam_any(n) => false,
        (_, Val::Ne(n)) if lam_any(n) => false,
        // Solving is top-level: the rigid side may be any non-lambda value
        // (a universe, a Pi, an Id, ...), not just a neutral.
        (Val::Ne(n), _) if flex_head(n) => match &**n {
            Ne::Meta(m, sp) => solve(l, *m, sp, &u),
            _ => unreachable!("guarded by flex_head"),
        },
        (_, Val::Ne(n)) if flex_head(n) => match &**n {
            Ne::Meta(m2, sp2) => solve(l, *m2, sp2, &t),
            _ => unreachable!("guarded by flex_head"),
        },
        (Val::Ne(n), Val::Ne(n2)) => match (&**n, &**n2) {
            (Ne::Var(x, sp), Ne::Var(x2, sp2)) if x == x2 => unify_sp(l, sp, sp2),
            (Ne::Data(d, sp), Ne::Data(d2, sp2)) if d == d2 => unify_sp(l, sp, sp2),
            (Ne::Ctor(d, i, sp), Ne::Ctor(d2, i2, sp2)) if d == d2 && i == i2 => {
                unify_sp(l, sp, sp2)
            }
            (Ne::Elim(d, sp), Ne::Elim(d2, sp2)) if d == d2 => unify_sp(l, sp, sp2),
            _ => false,
        },
        _ => false,
    }
}

/// Whether the neutral is headed by a metavariable.
fn flex_head(n: &Ne) -> bool {
    matches!(n, Ne::Meta(..))
}

/// Whether the neutral is a stuck lambda with only variable patterns.
fn lam_var_patterns(n: &Ne) -> bool {
    match n {
        Ne::Lam(ps, _, _, _) => all_var_patterns(ps),
        _ => false,
    }
}

/// Whether the neutral is a stuck lambda at all.
fn lam_any(n: &Ne) -> bool {
    matches!(n, Ne::Lam(..))
}

fn all_var_patterns(ps: &[Pn]) -> bool {
    ps.iter().all(|p| matches!(p, Pn::Var(_)))
}

/// One eta step for the stuck-lambda cases of [`unify`].
fn eta_unify(l: Lvl, t: &Val, u: &Val) -> bool {
    let var = vec![Val::vvar(l)];
    unify(l.inc(), &napp(t, &var).val, &napp(u, &var).val)
}

/// Conversion where either side may solve: the comparison check positions
/// use.  Pure conversion goes first (it keeps cumulativity), and only a
/// failure with metavariables in scope falls back to unification, which
/// solves flex problems anywhere inside the values -- including a `Pi`
/// codomain, where conversion alone cannot make progress.
pub fn conv_or_unify(l: Lvl, t: &Val, u: &Val) -> bool {
    conv(l, t, u) || (!meta::is_empty() && unify(l, t, u))
}

/// [`conv_or_unify`] with cumulativity in the non-solving direction (the
/// comparison `check` uses against an expected type).
pub fn conv_or_unify_le(l: Lvl, t: &Val, u: &Val) -> bool {
    conv_le(l, t, u) || (!meta::is_empty() && unify(l, t, u))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_ty_drops_shifts_and_rejects() {
        // (A : U)(B : U) -> (C : U) -> A, pruning B: the codomain's reference
        // to A shifts from index 2 to 1.
        let ty = |body: Ix| {
            Tm::Pi(
                "A".into(),
                Box::new(Tm::U(0)),
                Box::new(Tm::Pi(
                    "B".into(),
                    Box::new(Tm::U(0)),
                    Box::new(Tm::Pi(
                        "C".into(),
                        Box::new(Tm::U(0)),
                        Box::new(Tm::Var(body)),
                    )),
                )),
            )
        };
        let pruned = prune_ty(&[true, false, true], &ty(Ix(2)));
        assert_eq!(
            pruned,
            Some(Tm::Pi(
                "A".into(),
                Box::new(Tm::U(0)),
                Box::new(Tm::Pi("C".into(), Box::new(Tm::U(0)), Box::new(Tm::Var(Ix(1)))))
            ))
        );
        // dropping A while the codomain mentions it is rejected
        assert_eq!(prune_ty(&[false, true, true], &ty(Ix(2))), None);
        // dropping A when nothing mentions it works, and C's index shifts
        let ty2 = Tm::Pi(
            "A".into(),
            Box::new(Tm::U(0)),
            Box::new(Tm::Pi(
                "B".into(),
                Box::new(Tm::U(0)),
                Box::new(Tm::Pi(
                    "C".into(),
                    Box::new(Tm::U(0)),
                    Box::new(Tm::Var(Ix(0))),
                )),
            )),
        );
        assert_eq!(
            prune_ty(&[false, true, true], &ty2),
            Some(Tm::Pi(
                "B".into(),
                Box::new(Tm::U(0)),
                Box::new(Tm::Pi("C".into(), Box::new(Tm::U(0)), Box::new(Tm::Var(Ix(0)))))
            ))
        );
        // a pruning longer than the chain is rejected
        assert!(prune_ty(&[true, true, true, true], &ty(Ix(2))).is_none());
    }

    #[test]
    fn lams_takes_names_from_the_type() {
        let ty = Tm::Pi(
            "A".into(),
            Box::new(Tm::U(0)),
            Box::new(Tm::Pi("x".into(), Box::new(Tm::Var(Ix(1))), Box::new(Tm::Var(Ix(0))))),
        );
        let l = lams(2, &ty, Tm::Var(Ix(0)));
        // λ A x . x -- the body's innermost binder is the wrapped lambda's own
        assert_eq!(
            l,
            Tm::Lam(
                vec![Pn::Var("A".into())],
                Box::new(Tm::Lam(vec![Pn::Var("x".into())], Box::new(Tm::Var(Ix(0)))))
            )
        );
    }
}
