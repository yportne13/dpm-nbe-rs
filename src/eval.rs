//! Evaluation: normalization by evaluation, forcing, reduction and quotation.
//!
//! Direct port of `Eval.hs`.  The central operation is *forcing* (`frc`): a
//! semantic value may be a delayed substitution (`Val::Sub`) or a stuck
//! neutral (`Val::Ne`), and forcing computes its weak head normal form.  A
//! lambda is itself a neutral (`Ne::Lam`) because a lambda is not stable under
//! substitution; it only reduces once a spine of arguments can be matched
//! against its patterns.

use crate::subst::{define, lookup_sub};
use crate::syntax::*;

use std::rc::Rc;

// ---------------------------------------------------------------------------
// Substitution actions (`SubAction`)
// ---------------------------------------------------------------------------

/// `subst s2 s1 = s1[s2] = s1 ∘ s2`
///
/// DEVIATION from the original: `Eval.hs` composes by mapping over the entries
/// of `s1` (`IM.map (subst s2) s`), which only stays cheap because Haskell is
/// lazy.  Under strict evaluation that recursion re-composes the same nested
/// values over and over -- `subst_sub` was called 2^n times while checking an
/// n-step transitivity proof -- so the port composes by concatenating layer
/// lists and resolves them on demand in `subst::lookup_sub`.
///
/// Also fixed: the original stores `dom = dom s1`; composing `s2 : Sub Ξ Γ`
/// with `s1 : Sub Γ Δ` yields `Sub Ξ Δ`, i.e. the domain of the *argument*.
pub fn subst_sub(s2: &Sub, s1: &Sub) -> Sub {
    if s1.dom.0 > s2.cod.0 {
        panic!("subst: (co)domain mismatch");
    }
    if s1.layers.is_empty() {
        return Sub {
            dom: s2.dom,
            cod: s1.cod,
            layers: s2.layers.clone(),
        };
    }
    if s2.layers.is_empty() {
        return Sub {
            dom: s2.dom,
            cod: s1.cod,
            layers: s1.layers.clone(),
        };
    }
    let mut layers = Vec::with_capacity(s2.layers.len() + s1.layers.len());
    layers.extend(s2.layers.iter().cloned());
    layers.extend(s1.layers.iter().cloned());
    Sub {
        dom: s2.dom,
        cod: s1.cod,
        layers,
    }
}

/// Substituting into a value always builds a *delayed* substitution.
pub fn subst_val(s: &Sub, v: &Val) -> Val {
    match v {
        Val::Sub(v2, s2) => Val::Sub(v2.clone(), Rc::new(subst_sub(s, s2))),
        _ => Val::Sub(Rc::new(v.clone()), Rc::new(s.clone())),
    }
}

/// Substitute in every element of a spine.
pub fn subst_sp(s: &Sub, sp: &Spine) -> Spine {
    sp.iter().map(|v| subst_val(s, v)).collect()
}

/// Substitute in every value of an environment.
pub fn subst_env(s: &Sub, env: &Env) -> Env {
    Env(env.0.iter().map(|v| subst_val(s, v)).collect())
}

/// A closure is substituted by substituting its environment only.
pub fn subst_cl(s: &Sub, cl: &Closure) -> Closure {
    Closure {
        env: subst_env(s, &cl.env),
        body: cl.body.clone(),
    }
}

// ---------------------------------------------------------------------------
// Forcing (`Force`: `frc` / `frcS`)
// ---------------------------------------------------------------------------

/// Force `v` to weak head normal form: resolve delayed substitutions, run
/// primitive reductions and apply stuck lambdas.
///
/// Forcing stops at the *head*: a rigid neutral -- a variable, a data former,
/// a constructor -- is in weak head normal form whatever its arguments are,
/// so its arguments are left untouched (they may be delayed substitutions)
/// and forced by whoever consumes them.  Eagerly walking the whole spine
/// instead, as an earlier version of the port did, makes a single `frc` cost
/// the size of the entire value: building `suc (suc ... zero)` numerals then
/// forces a depth-n chain once per eliminator step, i.e. quadratically in
/// the successor count.  (The original writes `frcS sb <$> sp` under these
/// heads as well, but laziness means it never runs.)
pub fn frc(v: &Val) -> Val {
    match v {
        Val::Sub(v, s) => frc_s(s, v),
        Val::Ne(n) if matches!(&**n, Ne::Var(..) | Ne::Data(..) | Ne::Ctor(..)) => v.clone(),
        Val::Ne(n) => frc_ne(n),
        _ => v.clone(),
    }
}

/// Forcing a reduction result: only descend if the step made progress,
/// otherwise we would re-run the same blocked reduction forever.
pub fn frc_res(r: &Res) -> Val {
    if r.progressed {
        frc(&r.val)
    } else {
        r.val.clone()
    }
}

/// `frcS sb v`: force `v` under the substitution `sb`.
pub fn frc_s(s: &Sub, v: &Val) -> Val {
    match v {
        Val::Sub(v2, s2) => frc_s(&subst_sub(s, s2), v2),
        Val::Ne(n) => frc_ne_s(s, n),
        Val::U(l) => Val::U(*l),
        Val::Pi(x, a, cl) => Val::Pi(
            x.clone(),
            Rc::new(subst_val(s, a)),
            Rc::new(subst_cl(s, cl)),
        ),
        Val::Id(a, x, y) => Val::Id(
            Rc::new(subst_val(s, a)),
            Rc::new(subst_val(s, x)),
            Rc::new(subst_val(s, y)),
        ),
        Val::Refl => Val::Refl,
        Val::Bot => Val::Bot,
    }
}

/// Force a neutral term: a stuck lambda is applied to its spine, the
/// eliminator is given a chance to fire.  Precondition: `n` is not a delayed
/// substitution.
pub fn frc_ne(n: &Ne) -> Val {
    match n {
        Ne::Sub(n2, s) => frc_ne_s(s, n2),
        // Rigid heads carry no pending substitution and cannot reduce, so
        // they are already in weak head normal form; their arguments are
        // left as they are and forced on demand (see [`frc`]).
        Ne::Var(..) | Ne::Data(..) | Ne::Ctor(..) => Val::ne(n.clone()),
        Ne::Lam(ps, k, cl, sp) if sp.is_empty() => {
            // A lambda is a semantic value: with no arguments to match there
            // is nothing to reduce, so it is already in weak head normal form.
            // (In the original this case loops: `frc` would call `napp` with an
            // empty spine, whose own `frc` calls `frc` again.)
            Val::ne(Ne::Lam(ps.clone(), *k, cl.clone(), Vec::new()))
        }
        Ne::Lam(ps, k, cl, sp) => {
            let lam = Val::ne(Ne::Lam(ps.clone(), *k, cl.clone(), Vec::new()));
            frc_res(&napp(&lam, &frc_sp(sp)))
        }
        // The eliminator forces the target itself, and the methods it applies
        // are forced by the `napp` that applies them, so the spine needs no
        // pre-forcing here either.
        Ne::Elim(d, sp) => frc_res(&elim_step(d, sp)),
    }
}

/// `frcS sb n`: force a neutral term under the substitution `sb`.
pub fn frc_ne_s(s: &Sub, n: &Ne) -> Val {
    match n {
        Ne::Sub(n2, s2) => frc_ne_s(&subst_sub(s, s2), n2),
        Ne::Var(v, sp) => frc_res(&napp(&lookup_sub(s, *v), &frc_sp_s(s, sp))),
        Ne::Lam(ps, k, cl, sp) => frc_res(&match_(ps, *k, &subst_cl(s, cl), &frc_sp_s(s, sp))),
        Ne::Data(d, sp) => Val::ne(Ne::Data(d.clone(), subst_sp(s, sp))),
        Ne::Ctor(d, i, sp) => Val::ne(Ne::Ctor(d.clone(), *i, subst_sp(s, sp))),
        Ne::Elim(d, sp) => frc_res(&elim_step(d, &subst_sp(s, sp))),
    }
}

/// Force every element of a spine.
pub fn frc_sp(sp: &Spine) -> Spine {
    sp.iter().map(frc).collect()
}

/// Force every element of a spine under the substitution s.
pub fn frc_sp_s(s: &Sub, sp: &Spine) -> Spine {
    sp.iter().map(|v| frc_s(s, v)).collect()
}

// ---------------------------------------------------------------------------
// Reduction of the eliminators
// ---------------------------------------------------------------------------

/// Try to fire the eliminator of `def` on the spine it has collected,
/// `[params.., motive, methods.., target, extra..]`.
///
/// It reduces when the spine is complete and the target is a saturated
/// constructor of the very same type:
///
/// ```text
/// elim D ps P ms.. (c_i ps as..)  ↦  m_i a_1 r_1 .. a_j r_j
/// ```
///
/// where each `r_j` (only for a recursive argument `a_j`) is the eliminator
/// re-applied to `a_j`.  Anything else -- a short spine, a neutral target, a
/// foreign or unsaturated constructor -- stays a stuck neutral.
pub fn elim_step(def: &Rc<IndDef>, sp: &Spine) -> Res {
    let k = def.num_params();
    let n = def.ctors.len();
    let arity = k + n + 2;
    let stuck = || block(Val::ne(Ne::Elim(def.clone(), sp.to_vec())));
    if sp.len() < arity {
        return stuck();
    }
    let target = frc(&sp[k + n + 1]);
    let (i, cargs) = match &target {
        Val::Ne(ne) => match &**ne {
            Ne::Ctor(d2, i, cargs) if Rc::ptr_eq(d2, def) => (*i, cargs.clone()),
            _ => return stuck(),
        },
        _ => return stuck(),
    };
    let ci = &def.ctors[i];
    if cargs.len() != k + ci.num_args {
        return stuck();
    }
    // The method, applied to the constructor's arguments in the order the
    // method type binds them: each argument, followed (for a recursive one) by
    // the eliminator re-applied to it, i.e. `a_1 r_1 .. a_j r_j`.
    let method = sp[k + 1 + i].clone();
    let mut margs: Spine = Vec::with_capacity(2 * ci.num_args);
    for j in 0..ci.num_args {
        let a = cargs[k + j].clone();
        margs.push(a.clone());
        if ci.recursive[j] {
            let mut rec_sp: Spine = sp[..k + n + 1].to_vec();
            rec_sp.push(a);
            let r = elim_step(def, &rec_sp);
            margs.push(frc_res(&r));
        }
    }
    let mut res = napp(&method, &margs);
    if sp.len() > arity {
        // Over-application: the arguments beyond the eliminator's own arity
        // were waiting for the reduced result.
        let extra: Spine = sp[arity..].to_vec();
        let over = napp(&frc_res(&res), &extra);
        res = Res {
            val: over.val,
            progressed: true,
        };
    }
    progress(frc_res(&res))
}

// ---------------------------------------------------------------------------
// Application and pattern matching on neutrals
// ---------------------------------------------------------------------------

/// Apply a value to a spine of arguments.
pub fn napp(v: &Val, sp: &Spine) -> Res {
    let v = frc(v);
    if sp.is_empty() {
        return progress(v);
    }
    match &v {
        Val::Ne(n) => match &**n {
            Ne::Var(x, sp0) => {
                let mut args = sp0.clone();
                args.extend_from_slice(sp);
                block(Val::ne(Ne::Var(*x, args)))
            }
            Ne::Data(d, sp0) => {
                let mut args = sp0.clone();
                args.extend_from_slice(sp);
                block(Val::ne(Ne::Data(d.clone(), args)))
            }
            Ne::Ctor(d, i, sp0) => {
                let mut args = sp0.clone();
                args.extend_from_slice(sp);
                block(Val::ne(Ne::Ctor(d.clone(), *i, args)))
            }
            Ne::Lam(ps, k, cl, sp0) => {
                let mut args = sp0.clone();
                args.extend_from_slice(sp);
                if args.len() >= k.0 {
                    match_(ps, *k, cl, &args)
                } else {
                    block(Val::ne(Ne::Lam(ps.clone(), *k, cl.clone(), args)))
                }
            }
            Ne::Elim(d, sp0) => {
                let mut args = sp0.clone();
                args.extend_from_slice(sp);
                elim_step(d, &args)
            }
            Ne::Sub(..) => panic!("napp: impossible (unforced neutral)"),
        },
        _ => panic!("napp: impossible (applying a non-neutral value)"),
    }
}

/// Instantiate a closure with exactly its arguments.  The arity is not
/// needed: the caller guarantees `sp.len() == num_pvars(ps)`.
pub fn capp(cl: &Closure, sp: &Spine) -> Val {
    eval(&cl.env.extend(sp), &cl.body)
}

/// Match a spine of arguments against the patterns of a stuck lambda.
///
/// Every pattern consumes exactly one argument, so when matching gets stuck we
/// re-create the neutral from the *original* patterns and spine.  Re-running
/// the whole match when more arguments arrive then recomputes the already
/// matched binders, which is what makes partial application correct (the
/// original instead kept the remaining patterns next to the original spine,
/// losing the arguments consumed so far).
pub fn match_(ps: &[Pn], n: Lvl, cl: &Closure, sp: &Spine) -> Res {
    let mut acc: Spine = Vec::new();
    let mut i = 0usize; // index into the patterns
    let mut j = 0usize; // index into the arguments
    loop {
        if i >= ps.len() {
            // All patterns matched: instantiate the closure and apply the rest.
            let r = napp(&capp(cl, &acc), &sp[j..].to_vec());
            return progress(frc_res(&r));
        }
        if j >= sp.len() {
            return block(stuck_lam(ps, n, cl, sp));
        }
        match &ps[i] {
            Pn::Var(_) => {
                acc.push(frc(&sp[j]));
                i += 1;
                j += 1;
            }
            Pn::Refl => {
                if matches!(frc(&sp[j]), Val::Refl) {
                    i += 1;
                    j += 1;
                } else {
                    return block(stuck_lam(ps, n, cl, sp));
                }
            }
            Pn::Abs => return block(stuck_lam(ps, n, cl, sp)),
        }
    }
}

/// A lambda that cannot reduce yet, together with the arguments it has
/// received so far (`len(spine) < num_pvars(ps)`).
fn stuck_lam(ps: &[Pn], n: Lvl, cl: &Closure, sp: &Spine) -> Val {
    Val::ne(Ne::Lam(ps.to_vec(), n, cl.clone(), sp.to_vec()))
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// Evaluate a core term in an environment.  Precondition: `t` is a term of
/// the context `env`, and `Tm::Bot` only ever occurs under an absurd pattern.
pub fn eval(env: &Env, t: &Tm) -> Val {
    match t {
        Tm::Var(x) => lookup_env(*x, env),
        Tm::App(t, u) => napp(&eval(env, t), &vec![eval(env, u)]).val,
        Tm::Lam(ps, t) => Val::ne(Ne::Lam(
            ps.clone(),
            num_pvars(ps),
            Closure::new(env.clone(), (**t).clone()),
            Vec::new(),
        )),
        Tm::Pi(x, a, b) => Val::pi(
            x.clone(),
            eval(env, a),
            Closure::new(env.clone(), (**b).clone()),
        ),
        Tm::Let(_, _, t, u) => {
            let tv = eval(env, t);
            eval(&define(env, tv), u)
        }
        Tm::U(l) => Val::U(*l),
        Tm::Id(a, x, y) => Val::id(eval(env, a), eval(env, x), eval(env, y)),
        Tm::Refl => Val::Refl,
        Tm::Data(def) => Val::data(def),
        Tm::Ctor(def, i) => Val::ctor(def, *i),
        Tm::Elim(def) => Val::elim(def),
        // DEVIATION 7: `Bot` is the (unreachable) body of an absurd pattern.
        // The original panics here, which makes an absurd-pattern lambda crash
        // as soon as it is evaluated, e.g. `let C : ... = λ A (!); C`.
        Tm::Bot => Val::Bot,
    }
}

// ---------------------------------------------------------------------------
// Conversion checking (beta only; eta is deliberately absent -- NOTES.md,
// deviation 13)
// ---------------------------------------------------------------------------

/// Cumulative subtyping: like [`conv`], except that a universe `U i` is
/// below `U j` whenever `i <= j`, and a function type may return a *smaller*
/// type than expected (domains must agree).  This is the comparison to use
/// where a term is *checked against* an expected type; definitional equality
/// (`refl`, unification) stays strict.
pub fn conv_le(l: Lvl, t: &Val, u: &Val) -> bool {
    let t = frc(t);
    let u = frc(u);
    match (&t, &u) {
        (Val::U(i), Val::U(j)) => i <= j,
        (Val::Pi(_, a, b), Val::Pi(_, a2, b2)) => {
            conv(l, a, a2) && conv_le(l.inc(), &b.apply(Val::vvar(l)), &b2.apply(Val::vvar(l)))
        }
        _ => conv(l, &t, &u),
    }
}

/// Beta conversion (no eta; see NOTES.md, deviation 13).  Precondition:
/// both values have the same type.
pub fn conv(l: Lvl, t: &Val, u: &Val) -> bool {
    let t = frc(t);
    let u = frc(u);
    match (&t, &u) {
        (Val::U(i), Val::U(j)) => i == j,
        (Val::Pi(_, a, b), Val::Pi(_, a2, b2)) => {
            conv(l, a, a2) && conv(l.inc(), &b.apply(Val::vvar(l)), &b2.apply(Val::vvar(l)))
        }
        (Val::Id(a, x, y), Val::Id(a2, x2, y2)) => {
            conv(l, a, a2) && conv(l, x, x2) && conv(l, y, y2)
        }
        (Val::Refl, Val::Refl) => true,
        (Val::Ne(n), Val::Ne(n2)) => conv_ne(l, n, n2),
        (Val::Sub(..), _) | (_, Val::Sub(..)) => {
            panic!("conv: impossible (unforced substitution)")
        }
        (Val::Bot, Val::Bot) => true,
        _ => false,
    }
}

/// Compare two spines pointwise.
pub fn conv_sp(l: Lvl, sp: &Spine, sp2: &Spine) -> bool {
    let sp = frc_sp(sp);
    let sp2 = frc_sp(sp2);
    sp.len() == sp2.len() && sp.iter().zip(sp2.iter()).all(|(a, b)| conv(l, a, b))
}

/// `n` and `n'` must already be forced.
pub fn conv_ne(l: Lvl, n: &Ne, n2: &Ne) -> bool {
    match (n, n2) {
        (Ne::Var(v, sp), Ne::Var(v2, sp2)) => v == v2 && conv_sp(l, sp, sp2),
        (Ne::Lam(_, k, cl, sp), Ne::Lam(_, _, cl2, sp2)) => {
            conv_cl(l, *k, cl, cl2) && conv_sp(l, sp, sp2)
        }
        // Data types, constructors and stuck eliminators are rigid heads:
        // two of them are convertible when they are the same head applied to
        // convertible spines.  (For stuck `elim` this mirrors the original's
        // `plus` case, which the Haskell `convNe` was missing -- NOTES.md
        // deviation 13: conversion must at least be reflexive.)
        (Ne::Data(d, sp), Ne::Data(d2, sp2)) => d == d2 && conv_sp(l, sp, sp2),
        (Ne::Ctor(d, i, sp), Ne::Ctor(d2, i2, sp2)) => {
            d == d2 && i == i2 && conv_sp(l, sp, sp2)
        }
        (Ne::Elim(d, sp), Ne::Elim(d2, sp2)) => d == d2 && conv_sp(l, sp, sp2),
        _ => false,
    }
}

/// `VVar l, VVar (l+1), ..., VVar (l+n-1)`
pub fn new_vars(l: Lvl, n: Lvl) -> Spine {
    (0..n.0).map(|i| Val::vvar(l.plus(i))).collect()
}

/// Compare two closures by opening them with `n` fresh variables.
pub fn conv_cl(l: Lvl, n: Lvl, cl1: &Closure, cl2: &Closure) -> bool {
    let vs = new_vars(l, n);
    conv(l.plus(n.0), &capp(cl1, &vs), &capp(cl2, &vs))
}

/// A closure opened with `n` fresh variables.
pub fn free_closure(l: Lvl, n: Lvl, cl: &Closure) -> Val {
    capp(cl, &new_vars(l, n))
}

// ---------------------------------------------------------------------------
// Quotation (read-back)
// ---------------------------------------------------------------------------

/// Collapse the outermost delayed substitution of a neutral term.
pub fn un_sub_ne(n: &Ne) -> Ne {
    match n {
        Ne::Sub(n2, s) => un_sub_ne_s(s, n2),
        _ => n.clone(),
    }
}

/// unSubNeS: collapse a delayed substitution into a neutral term.
pub fn un_sub_ne_s(sub: &Sub, n: &Ne) -> Ne {
    match n {
        Ne::Sub(n2, s2) => un_sub_ne_s(&subst_sub(sub, s2), n2),
        Ne::Var(x, sp) => Ne::Var(*x, subst_sp(sub, sp)),
        Ne::Lam(ps, k, cl, sp) => Ne::Lam(ps.clone(), *k, subst_cl(sub, cl), subst_sp(sub, sp)),
        Ne::Data(d, sp) => Ne::Data(d.clone(), subst_sp(sub, sp)),
        Ne::Ctor(d, i, sp) => Ne::Ctor(d.clone(), *i, subst_sp(sub, sp)),
        Ne::Elim(d, sp) => Ne::Elim(d.clone(), subst_sp(sub, sp)),
    }
}

/// Read a value back to a core term at level l.  Precondition: every free
/// variable of  is below l, and  is not a delayed substitution.
pub fn quote_val(l: Lvl, v: &Val) -> Tm {
    match frc(v) {
        Val::Ne(n) => quote_ne(l, &n),
        Val::Sub(..) => panic!("quote: impossible (unforced substitution)"),
        Val::U(l) => Tm::U(l),
        Val::Pi(x, a, cl) => Tm::Pi(
            x,
            Box::new(quote_val(l, &a)),
            Box::new(quote_val(l.inc(), &free_closure(l, Lvl::ONE, &cl))),
        ),
        Val::Id(a, x, y) => Tm::Id(
            Box::new(quote_val(l, &a)),
            Box::new(quote_val(l, &x)),
            Box::new(quote_val(l, &y)),
        ),
        Val::Refl => Tm::Refl,
        Val::Bot => Tm::Bot,
    }
}

/// Whether a prefix of the patterns matches the arguments supplied so far.
/// A `refl` pattern only matches `refl`, and an absurd pattern can never be
/// applied, so both are reported as "not matched".
fn prefix_matches(ps: &[Pn], sp: &[Val]) -> bool {
    debug_assert_eq!(ps.len(), sp.len());
    for (p, v) in ps.iter().zip(sp.iter()) {
        match p {
            Pn::Var(_) => {}
            Pn::Refl => {
                if !matches!(frc(v), Val::Refl) {
                    return false;
                }
            }
            Pn::Abs => return false,
        }
    }
    true
}

/// Read a neutral term back to a core term at level l.
pub fn quote_ne(l: Lvl, n: &Ne) -> Tm {
    match un_sub_ne(n) {
        Ne::Sub(..) => panic!("quote_ne: impossible (unforced substitution)"),
        Ne::Var(v, sp) => {
            let mut acc = Tm::Var(lvl2ix(l, v));
            for arg in &sp {
                acc = Tm::App(Box::new(acc), Box::new(quote_val(l, arg)));
            }
            acc
        }
        Ne::Lam(ps, n, cl, sp) => {
            // Quoting a lambda that has not reduced yet.
            //
            // If the arguments supplied so far match a prefix of the patterns,
            // the value is a *partial application*: complete the match with a
            // canonical argument for every remaining pattern (a fresh variable
            // for a variable pattern, `refl` for a refl pattern) and print the
            // remaining patterns as the binders of the normal form.  This is
            // the eta-expansion described in NOTES.md deviation 3, generalised
            // to pattern lists that mix variables and `refl`.
            let consumed = ps.len().min(sp.len());
            let splittable = !ps[consumed..].iter().any(|p| matches!(p, Pn::Abs));
            if consumed < ps.len() && splittable && prefix_matches(&ps[..consumed], &sp[..consumed])
            {
                let mut args = sp.clone();
                let mut fresh = 0usize;
                let missing: Vec<Pn> = ps[consumed..].to_vec();
                for p in &missing {
                    match p {
                        Pn::Var(_) => {
                            args.push(Val::vvar(l.plus(fresh)));
                            fresh += 1;
                        }
                        Pn::Refl => args.push(Val::Refl),
                        Pn::Abs => unreachable!("`splittable` excludes Pn::Abs"),
                    }
                }
                let res = match_(&ps, n, &cl, &args);
                if res.progressed {
                    return Tm::Lam(missing, Box::new(quote_val(l.plus(fresh), &res.val)));
                }
            }

            // Otherwise the value is stuck (a pattern does not match, or an
            // absurd pattern is reached).  There is no case-syntax in the core
            // language, so we print the lambda itself, opening the body with
            // fresh variables -- the original rendering, which preserves the
            // source pattern names.
            Tm::Lam(
                ps.clone(),
                Box::new(quote_val(l.plus(n.0), &capp(&cl, &new_vars(l, n)))),
            )
        }
        Ne::Data(d, sp) => quote_app(l, Tm::Data(d), &sp),
        Ne::Ctor(d, i, sp) => quote_app(l, Tm::Ctor(d, i), &sp),
        Ne::Elim(d, sp) => quote_app(l, Tm::Elim(d), &sp),
    }
}

/// A rigid head applied to a quoted spine.
fn quote_app(l: Lvl, head: Tm, sp: &Spine) -> Tm {
    let mut acc = head;
    for arg in sp {
        acc = Tm::App(Box::new(acc), Box::new(quote_val(l, arg)));
    }
    acc
}

/// Normalize a core term in an environment.
pub fn nf(env: &Env, t: &Tm) -> Tm {
    quote_val(env.len(), &eval(env, t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subst::{id_sub, insert_sub};
    use crate::testdata::nat_def;

    /// The numeral `k` as a saturated constructor value.
    ///
    /// Every call allocates a fresh `Nat` definition on purpose: values built
    /// from distinct declarations are compared by pointer, so tests that need
    /// one shared `Nat` bind their own `def` and use [`num_with`].
    fn num(k: usize) -> Val {
        let def = nat_def();
        num_with(&def, k)
    }

    fn num_with(def: &Rc<IndDef>, k: usize) -> Val {
        let mut v = Val::ctor(def, 0);
        for _ in 0..k {
            v = frc(&Val::ne(Ne::Ctor(def.clone(), 1, vec![v])));
        }
        v
    }

    /// Quote `v` at level `l` using the given names.
    fn at(l: Lvl, names: &[&str], v: &Val) -> String {
        let ns: Vec<String> = names.iter().map(|s| s.to_string()).collect();
        crate::pretty::pretty_tm(0, &ns, &quote_val(l, v))
    }

    fn closed(v: &Val) -> String {
        at(Lvl::ZERO, &[], v)
    }

    // -- forcing -----------------------------------------------------------

    #[test]
    fn forcing_identity_substitution_preserves_the_value() {
        // Substitution is applied lazily: `frcS` on a constructor only delays
        // the substitution on the spine, so equality is up to conversion.
        let def = nat_def();
        for v in [Val::U(0), num_with(&def, 0), num_with(&def, 3), Val::vvar(Lvl(0))] {
            assert!(conv(
                Lvl(1),
                &frc(&Val::vsub(v.clone(), id_sub(Lvl(1)))),
                &v
            ));
        }
        // Atoms come back exactly.
        let z = num_with(&def, 0);
        assert_eq!(frc(&Val::vsub(z.clone(), id_sub(Lvl(1)))), z);
        assert_eq!(frc(&Val::vsub(Val::U(0), id_sub(Lvl(1)))), Val::U(0));
        let d = Val::data(&def);
        assert_eq!(frc(&Val::vsub(d.clone(), id_sub(Lvl(1)))), d);
    }

    #[test]
    fn forcing_resolves_a_delayed_substitution() {
        let s = insert_sub(Lvl(1), Lvl(0), num(2));
        assert_eq!(
            closed(&frc(&Val::vsub(Val::vvar(Lvl(0)), s))),
            "suc (suc zero)"
        );
    }

    #[test]
    fn forcing_is_idempotent_on_a_stuck_lambda() {
        let lam = eval(
            &Env::empty(),
            &Tm::Lam(vec![Pn::Var("x".into())], Box::new(Tm::Var(Ix(0)))),
        );
        assert_eq!(frc(&lam), lam);
        assert_eq!(frc(&frc(&lam)), lam);
    }

    // -- the eliminator ----------------------------------------------------

    /// The spine `[motive, zero-case, suc-case, target]` for `Nat`, with a
    /// constant motive (its value is irrelevant to reduction).
    fn nat_elim_spine(mz: Val, ms: Val, target: Val) -> Spine {
        vec![Val::U(0), mz, ms, target]
    }

    /// `λ n r . suc r` as a value (the `suc` case of addition-like recursors).
    fn suc_case(def: &Rc<IndDef>) -> Val {
        let body = Tm::Ctor(def.clone(), 1);
        // λ n r . body (Var 0)
        let tm = Tm::Lam(
            vec![Pn::Var("n".into()), Pn::Var("r".into())],
            Box::new(Tm::App(Box::new(body), Box::new(Tm::Var(Ix(0))))),
        );
        eval(&Env::empty(), &tm)
    }

    #[test]
    fn elim_reduces_on_a_constructor_target() {
        let def = nat_def();
        // elim Nat U mz ms target  with  mz = 3,  ms = λ n r . suc r
        // computes  target + 3.
        let mz = num_with(&def, 3);
        for (target, expect) in [(0usize, 3usize), (2, 5), (4, 7)] {
            let sp = nat_elim_spine(mz.clone(), suc_case(&def), num_with(&def, target));
            let r = elim_step(&def, &sp);
            assert!(r.progressed, "should reduce for target = {target}");
            assert_eq!(
                closed(&frc_res(&r)),
                at(Lvl::ZERO, &[], &num_with(&def, expect)),
                "wrong result for {target} + 3"
            );
        }
    }

    #[test]
    fn elim_is_stuck_on_a_variable() {
        let def = nat_def();
        let sp = nat_elim_spine(num(0), suc_case(&def), Val::vvar(Lvl(0)));
        let r = elim_step(&def, &sp);
        assert!(!r.progressed);
        assert_eq!(
            at(Lvl(1), &["x"], &r.val),
            "elim Nat U zero (λ n r . suc r) x"
        );
    }

    #[test]
    fn elim_is_blocked_on_a_short_spine() {
        let def = nat_def();
        // A spine without the target argument is a partial application.
        let sp: Spine = nat_elim_spine(num(0), suc_case(&def), num(1))[..3].to_vec();
        let r = elim_step(&def, &sp);
        assert!(!r.progressed);
        match &r.val {
            Val::Ne(n) => assert!(matches!(&**n, Ne::Elim(..)), "got {n:?}"),
            other => panic!("expected a stuck elim, got {other:?}"),
        }
    }

    #[test]
    fn elim_fires_through_napp_on_the_last_argument() {
        let def = nat_def();
        let head = Val::elim(&def);
        let sp3 = nat_elim_spine(num_with(&def, 1), suc_case(&def), num_with(&def, 2))[..3]
            .to_vec();
        // Partially apply, then deliver the target.
        let partial = napp(&head, &sp3);
        assert!(!partial.progressed);
        let full = napp(&partial.val, &vec![num_with(&def, 1)]);
        assert!(full.progressed);
        assert_eq!(closed(&frc_res(&full)), "suc (suc zero)");
    }

    // -- application and pattern matching ----------------------------------

    #[test]
    fn applying_a_lambda_substitutes_its_argument() {
        // \x. x  applied to 2
        let lam = eval(
            &Env::empty(),
            &Tm::Lam(vec![Pn::Var("x".into())], Box::new(Tm::Var(Ix(0)))),
        );
        let r = napp(&lam, &vec![num(2)]);
        assert!(r.progressed);
        assert_eq!(closed(&frc_res(&r)), "suc (suc zero)");
    }

    #[test]
    fn partial_application_is_eta_expanded() {
        // \x y. elim Nat x zero y  applied to (suc zero): the eliminator is
        // stuck on the variable `y` and quotes as a stuck application, so the
        // value is the eta-expanded lambda keeping its source pattern name.
        let def = nat_def();
        let lam = eval(
            &Env::empty(),
            &Tm::Lam(
                vec![Pn::Var("x".into()), Pn::Var("y".into())],
                Box::new(Tm::App(
                    Box::new(Tm::App(
                        Box::new(Tm::App(
                            Box::new(Tm::Elim(def.clone())),
                            Box::new(Tm::Var(Ix(1))),
                        )),
                        Box::new(Tm::Ctor(def.clone(), 0)),
                    )),
                    Box::new(Tm::Var(Ix(0))),
                )),
            ),
        );
        let r = napp(&lam, &vec![num(1)]);
        assert!(!r.progressed, "the eliminator is stuck on the variable y");
        let printed = at(Lvl(1), &[], &r.val);
        assert!(
            printed.starts_with("λ y . elim Nat (suc zero) zero y"),
            "unexpected normal form: {printed}"
        );
    }

    #[test]
    fn a_stuck_refl_pattern_does_not_loop() {
        // \refl. refl  applied to a variable: cannot match, stays neutral
        let lam = eval(&Env::empty(), &Tm::Lam(vec![Pn::Refl], Box::new(Tm::Refl)));
        let r = napp(&lam, &vec![Val::vvar(Lvl(0))]);
        assert!(!r.progressed);
        assert_eq!(at(Lvl(1), &["x"], &r.val), "λ refl . refl");
    }

    #[test]
    fn match_consumes_one_argument_per_pattern() {
        let cl = Closure::new(Env::empty(), Tm::Bot);
        // the body is unreachable, so matching always succeeds structurally
        assert!(
            match_(
                &[Pn::Var("a".into()), Pn::Refl],
                Lvl(1),
                &cl,
                &vec![Val::Bot, Val::Refl]
            )
            .progressed
        );
    }

    // -- conversion --------------------------------------------------------

    #[test]
    fn conv_is_reflexive() {
        for v in [Val::U(0), num(0), num(4), Val::Refl, Val::data(&nat_def())] {
            assert!(conv(Lvl::ZERO, &v, &v));
        }
    }

    #[test]
    fn conv_reduces_before_comparing() {
        let def = nat_def();
        let two = num_with(&def, 2);
        let s = insert_sub(Lvl(1), Lvl(0), two.clone());
        let a = Val::vsub(Val::vvar(Lvl(0)), s);
        assert!(conv(Lvl(1), &a, &two));
    }

    #[test]
    fn conv_distinguishes_different_values() {
        assert!(!conv(Lvl::ZERO, &num(1), &num(2)));
        assert!(!conv(Lvl::ZERO, &Val::data(&nat_def()), &Val::U(0)));
    }

    #[test]
    fn conv_compares_functions_extensionally() {
        // \x. x  vs  \y. y
        let id = || {
            eval(
                &Env::empty(),
                &Tm::Lam(vec![Pn::Var("x".into())], Box::new(Tm::Var(Ix(0)))),
            )
        };
        assert!(conv(Lvl::ZERO, &id(), &id()));
    }

    #[test]
    fn two_declarations_of_the_same_shape_are_distinct_types() {
        // `data Nat` twice: structurally identical definitions, but nominal
        // typing says `Nat1` and `Nat2` are different types.
        assert_ne!(nat_def(), nat_def());
        assert!(!conv(Lvl::ZERO, &Val::data(&nat_def()), &Val::data(&nat_def())));
    }

    // -- quotation ---------------------------------------------------------

    #[test]
    fn quote_reads_back_normal_forms() {
        for t in [Tm::U(0), Tm::Refl, Tm::Bot] {
            assert_eq!(nf(&Env::empty(), &t), t);
        }
        // suc (suc zero) evaluates and quotes back
        let def = nat_def();
        let t = Tm::App(
            Box::new(Tm::Ctor(def.clone(), 1)),
            Box::new(Tm::App(
                Box::new(Tm::Ctor(def.clone(), 1)),
                Box::new(Tm::Ctor(def.clone(), 0)),
            )),
        );
        assert_eq!(closed(&eval(&Env::empty(), &t)), "suc (suc zero)");
    }

    #[test]
    fn quote_is_stable_under_re_evaluation() {
        let def = nat_def();
        let t = Tm::App(
            Box::new(Tm::Ctor(def.clone(), 1)),
            Box::new(Tm::Ctor(def.clone(), 0)),
        );
        let once = nf(&Env::empty(), &t);
        let twice = nf(&Env::empty(), &once);
        assert_eq!(once, twice);
    }

    // -- the value of `Bot` ------------------------------------------------

    #[test]
    fn bot_is_a_value() {
        // DEVIATION 7: the original has no case for `Bot` and panics.
        assert_eq!(eval(&Env::empty(), &Tm::Bot), Val::Bot);
        assert_eq!(frc(&Val::Bot), Val::Bot);
        assert!(conv(Lvl::ZERO, &Val::Bot, &Val::Bot));
        assert!(!conv(Lvl::ZERO, &Val::Bot, &Val::U(0)));
        assert_eq!(quote_val(Lvl::ZERO, &Val::Bot), Tm::Bot);
    }

    // -- substitution ------------------------------------------------------

    #[test]
    fn composing_after_solving_a_variable_narrows_the_domain() {
        // Mirrors `unifyS`: ɑ : Sub (g-1) g  composed with  γ : Sub g d
        let def = nat_def();
        let g = Lvl(2);
        let d = Lvl(2);
        let one = num_with(&def, 1);
        let alpha = insert_sub(g, Lvl(1), one.clone());
        let comp = subst_sub(&alpha, &id_sub(d));
        assert_eq!((comp.dom, comp.cod), (Lvl(1), d));
        let v = crate::subst::lookup_sub(&comp, Lvl(1));
        assert!(conv(g, &frc(&v), &one));

        // A second equation narrows the context again (this is what the
        // original's `dom = dom s1` got wrong).
        let alpha2 = insert_sub(Lvl(1), Lvl(0), num_with(&def, 2));
        let comp2 = subst_sub(&alpha2, &comp);
        assert_eq!((comp2.dom, comp2.cod), (Lvl(0), d));
    }

    #[test]
    fn substitution_is_delayed_then_forced() {
        let def = nat_def();
        let t = Tm::Var(Ix(0));
        let env = Env(vec![num_with(&def, 2)]);
        // Evaluating under the identity substitution changes nothing.
        assert_eq!(
            nf(&env, &t),
            Tm::App(
                Box::new(Tm::Ctor(def.clone(), 1)),
                Box::new(Tm::App(
                    Box::new(Tm::Ctor(def.clone(), 1)),
                    Box::new(Tm::Ctor(def, 0)),
                )),
            )
        );
    }
}
