//! Elaboration: bidirectional type checking, dependent pattern matching and
//! the unification algorithm that the pattern matching generates.
//!
//! Direct port of `Elab.hs`.
//!
//! References (from the original project):
//! * Jesper Cockx, Dominique Devriese, Frank Piessens, *Pattern Matching
//!   Without K*;
//! * Ulf Norell, *Towards a Practical Programming Language Based on Dependent
//!   Type Theory*.
//!
//! Note: this is a "with-K" implementation, as in the original.

use crate::context::{subst_ctx, Ctx, M};
use crate::eval::{conv, eval, frc, quote_val, subst_cl, subst_sub, subst_val};
use crate::pretty::show_val;
use crate::subst::{id_sub, insert_sub};
use crate::syntax::*;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Unification for dependent pattern matching
// ---------------------------------------------------------------------------

/// Result of unifying a list of equations.
#[derive(Clone, Debug)]
pub enum UnifyRes {
    /// The equations hold under the substitution `sub : Sub lvl Δ`.
    Succ { lvl: Lvl, sub: Sub },
    /// The equations are contradictory, so an absurd pattern is allowed.
    Abs,
    /// Agda would say *"I am not sure if there should be a pattern refl ..."*.
    Idk,
}

/// `unify Δ vs`
pub fn unify(d: Lvl, vs: &[(Val, Val)]) -> UnifyRes {
    unify_s(d, d, &id_sub(d), vs)
}

/// `unifyS Δ Γ γ vs`.  `Δ` is kept for parity with the original signature; the
/// running context is `g`.
fn unify_s(_d: Lvl, g: Lvl, gamma: &Sub, vs: &[(Val, Val)]) -> UnifyRes {
    let ((v1, v2), rest) = match vs.split_first() {
        None => {
            return UnifyRes::Succ {
                lvl: g,
                sub: gamma.clone(),
            }
        }
        Some(x) => x,
    };
    let u = frc(v1);
    let w = frc(v2);
    // Constructors of the same data type are injective (unify the spines), and
    // two *distinct* constructors of the same type are contradictory.
    if let (Val::Ne(n1), Val::Ne(n2)) = (&u, &w) {
        if let (Ne::Ctor(d1, i1, sp1), Ne::Ctor(d2, i2, sp2)) = (&**n1, &**n2) {
            if Rc::ptr_eq(d1, d2) {
                if i1 != i2 {
                    return UnifyRes::Abs;
                }
                if sp1.len() != sp2.len() {
                    // Ill-typed input; refuse to guess.
                    return UnifyRes::Idk;
                }
                let mut eqs: Vec<(Val, Val)> = sp1
                    .iter()
                    .zip(sp2.iter())
                    .map(|(x, y)| (x.clone(), y.clone()))
                    .collect();
                eqs.extend(rest.iter().cloned());
                return unify_s(_d, g, gamma, &eqs);
            }
        }
    }
    match unify1(g, &u, &w) {
        UnifyRes::Succ { lvl: s, sub: a } => {
            let gamma2 = subst_sub(&a, gamma);
            let vs2: Vec<(Val, Val)> = rest
                .iter()
                .map(|(x, y)| (subst_val(&a, x), subst_val(&a, y)))
                .collect();
            unify_s(_d, s, &gamma2, &vs2)
        }
        UnifyRes::Abs => UnifyRes::Abs,
        UnifyRes::Idk => UnifyRes::Idk,
    }
}

/// `unify1 Γ u v`: unify a single equation.  The returned substitution maps a
/// variable of `Γ` to a value; note that when a variable is solved the
/// resulting context has level `g - 1`.
pub fn unify1(g: Lvl, u: &Val, v: &Val) -> UnifyRes {
    let u = frc(u);
    let v = frc(v);

    if let (Some(x), Some(y)) = (as_var(&u), as_var(&v)) {
        if x == y {
            return UnifyRes::Succ {
                lvl: g,
                sub: id_sub(g),
            };
        }
    }
    if let Some(x) = as_var(&u) {
        return UnifyRes::Succ {
            lvl: g.pred(),
            sub: insert_sub(g, x, v.clone()),
        };
    }
    if let Some(x) = as_var(&v) {
        return UnifyRes::Succ {
            lvl: g.pred(),
            sub: insert_sub(g, x, u.clone()),
        };
    }

    match (&u, &v) {
        (Val::Refl, Val::Refl) => UnifyRes::Succ {
            lvl: g,
            sub: id_sub(g),
        },
        // Constructors are decomposed by `unify_s` before `unify1` runs.
        _ if conv(g, &u, &v) => UnifyRes::Succ {
            lvl: g,
            sub: id_sub(g),
        },
        _ => UnifyRes::Idk,
    }
}

/// A neutral value is a variable iff it is an application-free `NVar`.
fn as_var(v: &Val) -> Option<Lvl> {
    match v {
        Val::Ne(n) => match &**n {
            Ne::Var(x, sp) if sp.is_empty() => Some(*x),
            _ => None,
        },
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Error messages
// ---------------------------------------------------------------------------

const NOT_SURE: &str =
    "I am not sure if there should be a pattern '{}' for equation between {} and {}.";
const SHOULD_NOT_BE: &str = "It should not be '{}' for equation between {} and {}, \
                             in fact you should try pattern '{}'.";
const EXPECT_ID: &str = "Expected an identity type for pattern '{}', but got:\n\n  {}";

fn report<T>(ctx: &Ctx, msg: String) -> M<T> {
    Err(ctx.error(msg))
}

// ---------------------------------------------------------------------------
// Type checking
// ---------------------------------------------------------------------------

/// Check `r` against `ty`, returning its core term.
pub fn check(ctx: &Ctx, r: &Raw, ty: &Val) -> M<Tm> {
    let ty = frc(ty);
    match r {
        Raw::SrcPos(pos, t) => check(&ctx.with_pos(pos.clone()), t, &ty),
        Raw::Lam(ps, t) => check_pm(ctx, ps, t, &ty),
        Raw::Let(x, a, t, u) => {
            let a = check(ctx, a, &Val::U)?;
            let va = eval(&ctx.env, &a);
            let t = check(ctx, t, &va)?;
            let vt = eval(&ctx.env, &t);
            let u = check(&ctx.define(x.clone(), vt, va), u, &ty)?;
            Ok(Tm::Let(x.clone(), Box::new(a), Box::new(t), Box::new(u)))
        }
        // `refl` only checks directly against an identity type; otherwise it
        // falls through to `infer`, which reports "Can't infer type for refl".
        Raw::Refl if matches!(ty, Val::Id(..)) => {
            let Val::Id(_, x, y) = &ty else {
                unreachable!("guarded by matches!")
            };
            if conv(ctx.lvl, x, y) {
                Ok(Tm::Refl)
            } else {
                report(
                    ctx,
                    format!(
                        "expecting {} equals to {}, but it is not",
                        show_val(ctx, x),
                        show_val(ctx, y)
                    ),
                )
            }
        }
        r => {
            let (t, ty2) = infer(ctx, r)?;
            if conv(ctx.lvl, &ty2, &ty) {
                Ok(t)
            } else {
                report(
                    ctx,
                    format!(
                        "type mismatch\n\nexpected type:\n\n  {}\n\ninferred type:\n\n  {}\n",
                        show_val(ctx, &ty),
                        show_val(ctx, &ty2)
                    ),
                )
            }
        }
    }
}

/// Check a lambda whose patterns are `ps` against `ty`.
pub fn check_pm(ctx: &Ctx, ps: &[Pn], t: &Raw, ty: &Val) -> M<Tm> {
    let ty = frc(ty);
    let (p, rest) = match ps.split_first() {
        None => return check(ctx, t, &ty),
        Some(x) => x,
    };
    let (a, b) = match &ty {
        Val::Pi(_, a, b) => (a, b),
        _ => return report(ctx, "Too many patterns.".to_string()),
    };
    match p {
        Pn::Var(x) => {
            let ctx2 = ctx.bind(x.clone(), (**a).clone());
            let body = check_pm(&ctx2, rest, t, &b.apply(Val::vvar(ctx.lvl)))?;
            Ok(wrap_lam(p, rest, body))
        }
        Pn::Refl => {
            // The domain must be *forced* before matching on it: after a `refl`
            // refinement `subst_ctx` wraps the types in `Val::Sub`, and the
            // original (`case frc a of VId a x y`) does the same.
            let dom = frc(a);
            let (x, y) = match &dom {
                Val::Id(_, x, y) => (x.clone(), y.clone()),
                _ => return report(ctx, fmt(EXPECT_ID, &["refl", &show_val(ctx, a)])),
            };
            match unify(ctx.lvl, &[(x.as_ref().clone(), y.as_ref().clone())]) {
                UnifyRes::Succ { sub, .. } => {
                    let ctx2 = subst_ctx(&sub, ctx);
                    let ty2 = subst_cl(&sub, b).apply(Val::Refl);
                    let body = check_pm(&ctx2, rest, t, &ty2)?;
                    Ok(wrap_lam(p, rest, body))
                }
                UnifyRes::Abs => report(
                    ctx,
                    fmt(
                        SHOULD_NOT_BE,
                        &["refl", &show_val(ctx, &x), &show_val(ctx, &y), "(!)"],
                    ),
                ),
                UnifyRes::Idk => report(
                    ctx,
                    fmt(NOT_SURE, &["refl", &show_val(ctx, &x), &show_val(ctx, &y)]),
                ),
            }
        }
        Pn::Abs => {
            let dom = frc(a);
            let (x, y) = match &dom {
                Val::Id(_, x, y) => (x.clone(), y.clone()),
                _ => return report(ctx, fmt(EXPECT_ID, &["(!)", &show_val(ctx, a)])),
            };
            match unify(ctx.lvl, &[(x.as_ref().clone(), y.as_ref().clone())]) {
                UnifyRes::Succ { .. } => report(
                    ctx,
                    fmt(
                        SHOULD_NOT_BE,
                        &["(!)", &show_val(ctx, &x), &show_val(ctx, &y), "refl"],
                    ),
                ),
                UnifyRes::Abs => {
                    if rest.is_empty() && matches!(t, Raw::Bot) {
                        // Absurd pattern: the branch is empty by construction.
                        Ok(Tm::Lam(vec![Pn::Abs], Box::new(Tm::Bot)))
                    } else {
                        report(
                            ctx,
                            "It's absurd pattern, you should have nothing on the right hand side."
                                .to_string(),
                        )
                    }
                }
                UnifyRes::Idk => report(
                    ctx,
                    fmt(NOT_SURE, &["(!)", &show_val(ctx, &x), &show_val(ctx, &y)]),
                ),
            }
        }
    }
}

/// Attach a freshly matched pattern to the (possibly nested) lambda body.
fn wrap_lam(p: &Pn, rest: &[Pn], e: Tm) -> Tm {
    match e {
        Tm::Lam(ps, rhs) => {
            let mut all = Vec::with_capacity(ps.len() + 1);
            all.push(p.clone());
            all.extend(ps);
            Tm::Lam(all, rhs)
        }
        other => {
            assert!(
                rest.is_empty(),
                "check_pm: impossible (malformed lambda body)"
            );
            Tm::Lam(vec![p.clone()], Box::new(other))
        }
    }
}

/// Infer the type of `r`.
pub fn infer(ctx: &Ctx, r: &Raw) -> M<(Tm, Val)> {
    match r {
        Raw::SrcPos(pos, t) => infer(&ctx.with_pos(pos.clone()), t),

        Raw::Var(x) => {
            let (ix, ty) = ctx.lookup(x)?;
            Ok((Tm::Var(ix), ty))
        }

        Raw::U => Ok((Tm::U, Val::U)),

        Raw::App(t, u) => {
            let (t, tty) = infer(ctx, t)?;
            // DEVIATION from the original: `Elab.hs` matches on the inferred
            // type without forcing it first, so a function whose stored type
            // became a delayed substitution (which happens after a `refl`
            // pattern refines the context) cannot be applied.  Forcing is
            // semantically transparent and makes such applications work.
            match frc(&tty) {
                Val::Pi(_, a, b) => {
                    let u = check(ctx, u, &a)?;
                    let res = b.apply(eval(&ctx.env, &u));
                    Ok((Tm::App(Box::new(t), Box::new(u)), res))
                }
                tty => report(
                    ctx,
                    format!(
                        "Expected a function type, instead inferred:\n\n  {}",
                        show_val(ctx, &tty)
                    ),
                ),
            }
        }

        Raw::Lam(..) => report(ctx, "Can't infer type for lambda expression".to_string()),

        Raw::Pi(x, a, b) => {
            let a = check(ctx, a, &Val::U)?;
            let va = eval(&ctx.env, &a);
            let b = check(&ctx.bind(x.clone(), va), b, &Val::U)?;
            Ok((Tm::Pi(x.clone(), Box::new(a), Box::new(b)), Val::U))
        }

        Raw::Let(x, ty, t, u) => {
            let ty = check(ctx, ty, &Val::U)?;
            let vty = eval(&ctx.env, &ty);
            let t = check(ctx, t, &vty)?;
            let vt = eval(&ctx.env, &t);
            let (u, uty) = infer(&ctx.define(x.clone(), vt, vty), u)?;
            Ok((
                Tm::Let(x.clone(), Box::new(ty), Box::new(t), Box::new(u)),
                uty,
            ))
        }

        Raw::Refl => report(ctx, "Can't infer type for refl".to_string()),

        Raw::Id(a, x, y) => {
            let a = check(ctx, a, &Val::U)?;
            let va = eval(&ctx.env, &a);
            let x = check(ctx, x, &va)?;
            let y = check(ctx, y, &va)?;
            Ok((Tm::Id(Box::new(a), Box::new(x), Box::new(y)), Val::U))
        }

        Raw::Data(..) => {
            let (x, params, ty, ctors, body) = match r {
                Raw::Data(x, ps, ty, cs, b) => (x, ps, ty, cs, b),
                _ => unreachable!("guarded by the match arm"),
            };
            elab_data(ctx, x, params, ty, ctors, body)
        }

        Raw::Elim(a) => {
            let (t, _) = infer(ctx, a)?;
            let v = frc(&eval(&ctx.env, &t));
            let def = match &v {
                Val::Ne(n) => match &**n {
                    Ne::Data(d, sp) if sp.is_empty() => d.clone(),
                    Ne::Data(..) => {
                        return report(
                            ctx,
                            "elim expects the data type itself, not an instance of it"
                                .to_string(),
                        )
                    }
                    _ => return report(ctx, "elim expects a data type".to_string()),
                },
                _ => return report(ctx, "elim expects a data type".to_string()),
            };
            let ty = eval(&Env::empty(), &elim_ty(&def));
            Ok((Tm::Elim(def), ty))
        }

        Raw::Bot => panic!("infer: impossible (Bot)"),
    }
}

/// Tiny `printf`-style helper: substitute `{}` placeholders left to right.
fn fmt(template: &str, args: &[&str]) -> String {
    args.iter()
        .fold(template.to_string(), |s, a| s.replacen("{}", a, 1))
}

// ---------------------------------------------------------------------------
// Inductive declarations
// ---------------------------------------------------------------------------

/// Elaborate `data D params : ty where c1 : t1; ... ; body`.
///
/// The declaration becomes a chain of transparent `let`s:
///
/// ```text
/// let D   : Pi params U        = Data;
/// let c_1 : close(t_1)         = Ctor D 1;
/// ...
/// body
/// ```
///
/// The eliminator is *not* a binding: `elim D` is a core term whose type is
/// generated on demand by [`elim_ty`].
pub fn elab_data(
    ctx: &Ctx,
    name: &Name,
    params: &[(Name, Raw)],
    ty_raw: &Raw,
    ctors: &[(Name, Raw)],
    body: &Raw,
) -> M<(Tm, Val)> {
    let k = params.len();

    // 1. Parameters, elaborated in the ambient context: the datatype being
    //    declared does not exist yet, so a parameter type cannot mention it.
    let mut pctx = ctx.clone();
    let mut ptys: Vec<(Name, Tm)> = Vec::with_capacity(k);
    for (x, a_raw) in params {
        let a = check(&pctx, a_raw, &Val::U)?;
        let va = eval(&pctx.env, &a);
        ptys.push((x.clone(), a));
        pctx = pctx.bind(x.clone(), va);
    }
    // Parameter p_i lives at level ctx.lvl + i.

    // 2. The type after the parameters must be `U` (no indexed families).
    let ty = check(&pctx, ty_raw, &Val::U)?;
    if !conv(pctx.lvl, &eval(&pctx.env, &ty), &Val::U) {
        return report(
            ctx,
            format!(
                "an inductive type must be declared in U, but {}'s type is\n\n  {}",
                name,
                show_val(&pctx, &frc(&eval(&pctx.env, &ty)))
            ),
        );
    }

    // 3. The datatype's own type `Pi params U`, and a definition skeleton
    //    (parameters and constructors are committed once checking is done).
    let mut d_ty = ty;
    for (x, a) in ptys.iter().rev() {
        d_ty = Tm::Pi(x.clone(), Box::new(a.clone()), Box::new(d_ty));
    }
    let d_ty_val = eval(&ctx.env, &d_ty);
    let mut def = Rc::new(IndDef {
        name: name.clone(),
        params: Vec::new(),
        ctors: Vec::new(),
    });

    // 4. Constructors, checked in `[ambient, D, params]`: the datatype is
    //    bound *outside* the parameters, so wrapping the parameter Pis around
    //    a checked constructor type moves no variable -- from inside the
    //    constructor the parameters are the innermost binders, the datatype
    //    the next one out, exactly as in the wrapped term.  Everything that
    //    can hold a clone of `def` lives inside this block, so that the `Rc`
    //    is uniquely owned again when the definition is committed.
    let (stored_params, infos): (Vec<(Name, Tm)>, Vec<CtorInfo>) = {
        let mut ctx_d = ctx.define(name.clone(), Val::data(&def), d_ty_val.clone());
        let mut cps: Vec<(Name, Tm)> = Vec::with_capacity(k);
        for (x, a_raw) in params {
            let a = check(&ctx_d, a_raw, &Val::U)?;
            let va = eval(&ctx_d.env, &a);
            cps.push((x.clone(), a));
            ctx_d = ctx_d.bind(x.clone(), va);
        }
        // The datatype lives at level ctx.lvl and parameter p_i at
        // ctx.lvl + 1 + i.
        let pvs: Spine = (0..k).map(|i| Val::vvar(Lvl(ctx.lvl.0 + 1 + i))).collect();
        let mut infos = Vec::with_capacity(ctors.len());
        for (cname, craw) in ctors {
            let tm = check(&ctx_d, craw, &Val::U)?;

            // Walk the constructor's telescope on values: collect the argument
            // domains and the result.
            let mut lvl = ctx_d.lvl;
            let mut tyv = eval(&ctx_d.env, &tm);
            let mut argdoms: Vec<Val> = Vec::new();
            loop {
                match frc(&tyv) {
                    Val::Pi(_, a, cl) => {
                        argdoms.push((*a).clone());
                        tyv = cl.apply(Val::vvar(lvl));
                        lvl = lvl.inc();
                    }
                    result => {
                        tyv = result;
                        break;
                    }
                }
            }

            // The result must be `D p_0 .. p_{k-1}` with the parameters in
            // order.
            let well_formed_result = match &frc(&tyv) {
                Val::Ne(n) => match &**n {
                    Ne::Data(d2, sp) => Rc::ptr_eq(d2, &def) && *sp == pvs,
                    _ => false,
                },
                _ => false,
            };
            if !well_formed_result {
                return report(
                    ctx,
                    format!(
                        "constructor {} must return {}{}",
                        cname,
                        name,
                        if k == 0 {
                            String::new()
                        } else {
                            format!(
                                " {}",
                                (0..k)
                                    .map(|i| ptys[i].0.clone())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            )
                        }
                    ),
                );
            }

            // Strict positivity, and which arguments are recursive.  An
            // argument of type `D params` is recursive; otherwise `D` may
            // still occur strictly positively as the final codomain of a
            // chain of function types (`Nat -> D` is fine, it just gets no
            // induction hypothesis), as long as it never occurs in a domain.
            // Any other occurrence is rejected.
            let mut recursive = Vec::with_capacity(argdoms.len());
            for a in &argdoms {
                match classify_arg(&frc(a), &def, &pvs, lvl) {
                    Some(is_rec) => recursive.push(is_rec),
                    None => {
                        return report(
                            ctx,
                            format!(
                                "constructor {} of {} is not strictly positive: {} may only occur as the whole type of an argument, or as the result of a function type",
                                cname, name, name
                            ),
                        )
                    }
                }
            }

            // The stored public type.  First, references to enclosing
            // bindings are replaced by their (closed) values, so the stored
            // type only ever mentions the datatype itself; then the
            // parameter Pis are re-bound around the term.  Their domains
            // (`cps`) are valid in `[ambient, D, p_0 .. p_{j-1}]`, exactly the
            // context the wrapped term gives them, and the constructor term
            // itself needs no adjustment (the datatype is bound outside the
            // parameters while checking).
            let tm = subst_outer(&tm, 0, k, ctx);
            let mut w = tm;
            for (x, a) in cps.iter().rev() {
                w = Tm::Pi(x.clone(), Box::new(a.clone()), Box::new(w));
            }
            // The only free variable of the stored type may now be the
            // datatype itself: a reference that survived `subst_outer` points
            // at a binder (e.g. an enclosing lambda's), which cannot be
            // closed later.
            if !free_vars_only_d(&w, 0) {
                return report(
                    ctx,
                    format!(
                        "constructor {} of {} must not depend on enclosing bindings",
                        cname, name
                    ),
                );
            }

            infos.push(CtorInfo {
                name: cname.clone(),
                ty: w,
                num_args: argdoms.len(),
                recursive,
            });
        }
        (cps, infos)
    };
    {
        let def = Rc::get_mut(&mut def)
            .expect("data declaration: the definition is uniquely owned");
        def.params = stored_params;
        def.ctors = infos;
    }

    // 5. The body, under the bindings of the datatype and its constructors.
    let mut ctx2 = ctx.define(name.clone(), Val::data(&def), d_ty_val);
    let mut binds: Vec<(Name, Tm, Tm)> = vec![(name.clone(), d_ty, Tm::Data(def.clone()))];
    for (i, ci) in def.ctors.iter().enumerate() {
        let cty = close_data(&ci.ty, &def);
        let cty_val = eval(&ctx2.env, &cty);
        ctx2 = ctx2.define(ci.name.clone(), Val::ctor(&def, i), cty_val);
        binds.push((ci.name.clone(), cty, Tm::Ctor(def.clone(), i)));
    }
    let (body_tm, body_ty) = infer(&ctx2, body)?;
    let tm = binds.into_iter().rev().fold(body_tm, |acc, (x, a, t)| {
        Tm::Let(x, Box::new(a), Box::new(t), Box::new(acc))
    });
    Ok((tm, body_ty))
}

/// Replace every reference to an enclosing binding in a checked constructor
/// type (valid in `[ambient, D, params]`) with that binding's closed value,
/// so that the stored type only ever mentions the datatype and the
/// parameters.  References to *binders* (whose values are fresh variables)
/// stay free, and are rejected later by the `free_vars_only_d` invariant.
///
/// The datatype itself and the parameters are at slots `k` and `k-1 .. 0`
/// from any constructor-type position; anything further out is ambient and
/// addressed by its [`Env`] index there.
fn subst_outer(t: &Tm, d: usize, k: usize, ctx: &Ctx) -> Tm {
    match t {
        Tm::Var(i) => {
            if i.0 < d || i.0 - d <= k {
                // bound inside the term, a parameter, or the datatype
                t.clone()
            } else {
                let v = lookup_env(Ix(i.0 - d - k - 1), &ctx.env);
                // The value is closed, but it is spliced into a context with
                // the datatype (and, after wrapping, the parameters) inside
                // it, so its free variables -- if it has any, i.e. if the
                // binding was really a binder -- shift by one.
                crate::subst::lift(&quote_val(ctx.lvl, &v), 0, 1)
            }
        }
        Tm::U | Tm::Refl | Tm::Bot | Tm::Data(_) | Tm::Ctor(..) | Tm::Elim(_) => t.clone(),
        Tm::App(f, a) => Tm::App(
            Box::new(subst_outer(f, d, k, ctx)),
            Box::new(subst_outer(a, d, k, ctx)),
        ),
        Tm::Id(a, x, y) => Tm::Id(
            Box::new(subst_outer(a, d, k, ctx)),
            Box::new(subst_outer(x, d, k, ctx)),
            Box::new(subst_outer(y, d, k, ctx)),
        ),
        Tm::Pi(x, a, b) => Tm::Pi(
            x.clone(),
            Box::new(subst_outer(a, d, k, ctx)),
            Box::new(subst_outer(b, d + 1, k, ctx)),
        ),
        Tm::Lam(ps, b) => Tm::Lam(
            ps.clone(),
            Box::new(subst_outer(b, d + num_pvars(ps).0, k, ctx)),
        ),
        Tm::Let(x, a, v, b) => Tm::Let(
            x.clone(),
            Box::new(subst_outer(a, d, k, ctx)),
            Box::new(subst_outer(v, d, k, ctx)),
            Box::new(subst_outer(b, d + 1, k, ctx)),
        ),
    }
}

/// Whether `D` occurs in a (quoted) term.  Constructor and eliminator heads of
/// *other* datatypes are opaque atoms here: their arguments appear in the
/// surrounding application spine and are visited as ordinary subterms.
fn occurs_data(t: &Tm, def: &Rc<IndDef>) -> bool {
    match t {
        Tm::Data(d) => Rc::ptr_eq(d, def),
        Tm::Var(_) | Tm::U | Tm::Refl | Tm::Bot | Tm::Ctor(..) | Tm::Elim(_) => false,
        Tm::App(f, a) => occurs_data(f, def) || occurs_data(a, def),
        Tm::Id(a, x, y) => {
            occurs_data(a, def) || occurs_data(x, def) || occurs_data(y, def)
        }
        Tm::Pi(_, a, b) => occurs_data(a, def) || occurs_data(b, def),
        Tm::Lam(_, b) => occurs_data(b, def),
        Tm::Let(_, a, v, b) => {
            occurs_data(a, def) || occurs_data(v, def) || occurs_data(b, def)
        }
    }
}

/// Classify a (forced) constructor argument type for strict positivity.
///
/// * `Some(true)`  -- the argument is `D params`, a recursive argument.
/// * `Some(false)` -- `D` does not occur, or occurs only as the final
///   codomain of a chain of function types (`Nat -> D`): a legal but
///   non-recursive argument, which the eliminator gives no hypothesis for.
/// * `None`        -- `D` occurs in a domain (or anywhere else it may not),
///   so the constructor is not strictly positive.
///
/// `l` is the level to quote at (one past the argument telescope walked so
/// far, so the domain values stay in scope).
fn classify_arg(v: &Val, def: &Rc<IndDef>, pvs: &Spine, l: Lvl) -> Option<bool> {
    match v {
        Val::Ne(n) => match &**n {
            // `D params` is recursive; `D` applied to anything else is a
            // non-uniform occurrence and falls through to the occurs check.
            Ne::Data(d2, sp) if Rc::ptr_eq(d2, def) && *sp == *pvs => Some(true),
            _ => {
                if occurs_data(&quote_val(l, v), def) {
                    None
                } else {
                    Some(false)
                }
            }
        },
        Val::Pi(_, a, cl) => {
            if occurs_data(&quote_val(l, a), def) {
                return None;
            }
            classify_arg(&frc(&cl.apply(Val::vvar(l))), def, pvs, l.inc())
        }
        _ => {
            if occurs_data(&quote_val(l, v), def) {
                None
            } else {
                Some(false)
            }
        }
    }
}

/// Whether every free variable of a stored (parameter-abstracted) constructor
/// type is the datatype itself.  At local depth `d` the datatype is `Var d`
/// and the binders of the stored type occupy the slots below, so any `Var i`
/// with `i > d` escapes to an ambient binding.
fn free_vars_only_d(t: &Tm, d: usize) -> bool {
    match t {
        Tm::Var(i) => i.0 <= d,
        Tm::U | Tm::Refl | Tm::Bot | Tm::Data(_) | Tm::Ctor(..) | Tm::Elim(_) => true,
        Tm::App(f, a) => free_vars_only_d(f, d) && free_vars_only_d(a, d),
        Tm::Id(a, x, y) => free_vars_only_d(a, d) && free_vars_only_d(x, d) && free_vars_only_d(y, d),
        Tm::Pi(_, a, b) => free_vars_only_d(a, d) && free_vars_only_d(b, d + 1),
        Tm::Lam(ps, b) => free_vars_only_d(b, d + num_pvars(ps).0),
        Tm::Let(_, a, v, b) => {
            free_vars_only_d(a, d) && free_vars_only_d(v, d) && free_vars_only_d(b, d + 1)
        }
    }
}

/// Close a stored constructor type by replacing the datatype (its only free
/// variable) with the data type constant.  The result is valid wherever the
/// declaration itself is in scope.
pub fn close_data(t: &Tm, def: &Rc<IndDef>) -> Tm {
    fn go(t: &Tm, d: usize, def: &Rc<IndDef>) -> Tm {
        match t {
            Tm::Var(i) if i.0 >= d => Tm::Data(def.clone()),
            Tm::Var(i) => Tm::Var(*i),
            Tm::U | Tm::Refl | Tm::Bot | Tm::Data(_) | Tm::Ctor(..) | Tm::Elim(_) => t.clone(),
            Tm::App(f, a) => Tm::App(Box::new(go(f, d, def)), Box::new(go(a, d, def))),
            Tm::Id(a, x, y) => Tm::Id(
                Box::new(go(a, d, def)),
                Box::new(go(x, d, def)),
                Box::new(go(y, d, def)),
            ),
            Tm::Pi(x, a, b) => Tm::Pi(
                x.clone(),
                Box::new(go(a, d, def)),
                Box::new(go(b, d + 1, def)),
            ),
            Tm::Lam(ps, b) => Tm::Lam(ps.clone(), Box::new(go(b, d + num_pvars(ps).0, def))),
            Tm::Let(x, a, v, b) => Tm::Let(
                x.clone(),
                Box::new(go(a, d, def)),
                Box::new(go(v, d, def)),
                Box::new(go(b, d + 1, def)),
            ),
        }
    }
    go(t, 0, def)
}

/// `head arg1 .. argn`, left-associated.
fn app(head: Tm, args: Vec<Tm>) -> Tm {
    args.into_iter().fold(head, |f, a| {
        Tm::App(Box::new(f), Box::new(a))
    })
}

/// `p_{k-1} .. p_0` as variables, with `p_t` at index `off + (k - 1 - t)`.
fn p_vars(k: usize, off: usize) -> Vec<Tm> {
    (0..k).map(|t| Tm::Var(Ix(off + (k - 1 - t)))).collect()
}

/// The consecutive leading `Pi` binders of a term, outermost first.
fn split_pis(t: &Tm) -> Vec<(&Name, &Tm)> {
    let mut out = Vec::new();
    let mut cur = t;
    while let Tm::Pi(x, a, b) = cur {
        out.push((x, a.as_ref()));
        cur = b;
    }
    out
}

/// The type of the eliminator generated for `def`, as a closed term:
///
/// ```text
/// elim D : (p_0 : P_0) .. (p_{k-1} : P_{k-1})
///         (P : D ps -> U)
///         (m_0 : Method_0) .. (m_{n-1} : Method_{n-1})
///         (x : D ps) -> P ps x
/// ```
///
/// where the method for constructor `c_i : Pi params Pi args. D params` is
///
/// ```text
/// (a_1 : A_1) (r_1 : P ps a_1)? .. (a_m : A_m) (r_m : P ps a_m)? -> P ps (c_i ps as)
/// ```
///
/// with an induction hypothesis `r_j` after every recursive argument `a_j`.
/// The argument domains are relocated from the stored constructor type with
/// [`remap_free`]; the bookkeeping says how many extra binders (the motive, the
/// earlier methods, and the earlier induction hypotheses) sit between a domain
/// and the parameters it mentions.
pub fn elim_ty(def: &Rc<IndDef>) -> Tm {
    let k = def.num_params();
    let n = def.ctors.len();

    // The eliminator's own parameter binders.  The stored parameter domains
    // live in `[.., D, p_0 .. p_{j-1}]`; here the same Pis re-bind those
    // parameters, so each keeps its index and `D` becomes the constant.
    let top_params: Vec<(Name, Tm)> = def
        .params
        .iter()
        .enumerate()
        .map(|(j, (x, a))| {
            let map: Vec<Option<usize>> = (0..j)
                .map(Some)
                .chain(std::iter::once(None))
                .collect();
            (x.clone(), remap_free(a, &map, def))
        })
        .collect();

    // The motive's type `(x : D ps) -> U` in the context `[ps]`: the motive
    // does *not* abstract the parameters again, they are the eliminator's own
    // leading arguments.  `D` becomes the constant; the stored parameter
    // domains are not needed here.
    let motive_ty = Tm::Pi(
        "_".to_string(),
        Box::new(app(Tm::Data(def.clone()), p_vars(k, 0))),
        Box::new(Tm::U),
    );

    // The methods, each in the context `[ps, P, m_0 .. m_{i-1}]`.
    let methods: Vec<Tm> = def
        .ctors
        .iter()
        .enumerate()
        .map(|(i, ci)| {
            let args = &split_pis(&ci.ty)[k..];
            let m = args.len();
            // r[x] = number of recursive arguments among the first x.
            let mut r = vec![0usize; m + 1];
            for x in 1..=m {
                r[x] = r[x - 1] + usize::from(ci.recursive[x - 1]);
            }
            // The result `P (c_i ps as)`.  Under all of the method's own
            // binders, the motive is `m + r[m] + i` slots out.
            let result = {
                let base = m + r[m] + i;
                let mut ctor_args = p_vars(k, base + 1);
                for l in 1..=m {
                    ctor_args.push(Tm::Var(Ix((m - l) + (r[m] - r[l - 1]))));
                }
                app(Tm::Var(Ix(base)), vec![app(Tm::Ctor(def.clone(), i), ctor_args)])
            };
            // Assemble the Pi chain inside out.
            let mut body = result;
            for j in (1..=m).rev() {
                if ci.recursive[j - 1] {
                    // r_j : P a_j, with `a_j` the innermost binder.  A
                    // wildcard argument keeps a wildcard hypothesis name.
                    let base = j + r[j - 1] + i;
                    let rname = if args[j - 1].0 == "_" {
                        "_".to_string()
                    } else {
                        format!("{}'", args[j - 1].0)
                    };
                    body = Tm::Pi(
                        rname,
                        Box::new(app(Tm::Var(Ix(base)), vec![Tm::Var(Ix(0))])),
                        Box::new(body),
                    );
                }
                // a_j's domain, relocated: earlier arguments shift by the
                // induction hypotheses inserted among them, the parameters by
                // the motive and the earlier methods.
                let map: Vec<Option<usize>> = (1..j)
                    .rev()
                    .map(|l| Some((j - 1 - l) + (r[j - 1] - r[l])))
                    .chain((0..k).map(|s| Some(j - 1 + r[j - 1] + i + 1 + s)))
                    .chain(std::iter::once(None))
                    .collect();
                let dom = remap_free(args[j - 1].1, &map, def);
                body = Tm::Pi(args[j - 1].0.clone(), Box::new(dom), Box::new(body));
            }
            body
        })
        .collect();

    // `(x : D ps) -> P x`, under `[ps, P, ms]`.
    let tail = Tm::Pi(
        "x".to_string(),
        Box::new(app(Tm::Data(def.clone()), p_vars(k, n + 1))),
        Box::new(app(Tm::Var(Ix(n + 1)), vec![Tm::Var(Ix(0))])),
    );
    let mut out = tail;
    for ty in methods.into_iter().rev() {
        // The binder is anonymous: the domain (the method's result type)
        // already names the constructor it handles.
        out = Tm::Pi("_".to_string(), Box::new(ty), Box::new(out));
    }
    out = Tm::Pi("P".to_string(), Box::new(motive_ty), Box::new(out));
    for (x, a) in top_params.into_iter().rev() {
        out = Tm::Pi(x, Box::new(a), Box::new(out));
    }
    out
}

/// Rewrite the free variables of a stored-constructor-type fragment according
/// to `map`: slot `q` (counting the fragment's context from the innermost)
/// becomes `Some(target index above the local binders)` or `None` for the
/// datatype, which becomes the constant.  Local binders are left alone.
fn remap_free(t: &Tm, map: &[Option<usize>], def: &Rc<IndDef>) -> Tm {
    fn go(t: &Tm, d: usize, map: &[Option<usize>], def: &Rc<IndDef>) -> Tm {
        match t {
            Tm::Var(i) => {
                if i.0 < d {
                    Tm::Var(*i)
                } else {
                    match map[i.0 - d] {
                        Some(p) => Tm::Var(Ix(d + p)),
                        None => Tm::Data(def.clone()),
                    }
                }
            }
            Tm::U | Tm::Refl | Tm::Bot | Tm::Data(_) | Tm::Ctor(..) | Tm::Elim(_) => t.clone(),
            Tm::App(f, a) => Tm::App(Box::new(go(f, d, map, def)), Box::new(go(a, d, map, def))),
            Tm::Id(a, x, y) => Tm::Id(
                Box::new(go(a, d, map, def)),
                Box::new(go(x, d, map, def)),
                Box::new(go(y, d, map, def)),
            ),
            Tm::Pi(x, a, b) => Tm::Pi(
                x.clone(),
                Box::new(go(a, d, map, def)),
                Box::new(go(b, d + 1, map, def)),
            ),
            Tm::Lam(ps, b) => Tm::Lam(
                ps.clone(),
                Box::new(go(b, d + num_pvars(ps).0, map, def)),
            ),
            Tm::Let(x, a, v, b) => Tm::Let(
                x.clone(),
                Box::new(go(a, d, map, def)),
                Box::new(go(v, d, map, def)),
                Box::new(go(b, d + 1, map, def)),
            ),
        }
    }
    go(t, 0, map, def)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata::{list_def, nat_def};
    use crate::syntax::Ne;

    fn num(def: &Rc<IndDef>, k: usize) -> Val {
        let mut v = Val::ctor(def, 0);
        for _ in 0..k {
            v = frc(&Val::ne(Ne::Ctor(def.clone(), 1, vec![v])));
        }
        v
    }

    /// `λ n r . suc r` as a value.
    fn suc_case(def: &Rc<IndDef>) -> Val {
        eval(
            &Env::empty(),
            &Tm::Lam(
                vec![Pn::Var("n".into()), Pn::Var("r".into())],
                Box::new(Tm::App(
                    Box::new(Tm::Ctor(def.clone(), 1)),
                    Box::new(Tm::Var(Ix(0))),
                )),
            ),
        )
    }

    #[test]
    fn unifying_a_variable_with_itself_succeeds() {
        let r = unify(Lvl(2), &[(Val::vvar(Lvl(0)), Val::vvar(Lvl(0)))]);
        assert!(matches!(r, UnifyRes::Succ { lvl, .. } if lvl == Lvl(2)));
    }

    #[test]
    fn unifying_distinct_variables_solves_the_first() {
        let r = unify1(Lvl(2), &Val::vvar(Lvl(1)), &Val::vvar(Lvl(0)));
        match r {
            UnifyRes::Succ { lvl, sub } => {
                assert_eq!(lvl, Lvl(1));
                assert_eq!((sub.dom, sub.cod), (Lvl(1), Lvl(2)));
                assert_eq!(crate::subst::lookup_sub(&sub, Lvl(1)), Val::vvar(Lvl(0)));
            }
            other => panic!("expected Succ, got {other:?}"),
        }
    }

    #[test]
    fn unifying_a_variable_with_a_value_solves_it() {
        // `VVar 1 = zero` in a context of size 2
        let def = nat_def();
        let r = unify1(Lvl(2), &Val::vvar(Lvl(1)), &num(&def, 0));
        match r {
            UnifyRes::Succ { lvl, sub } => {
                assert_eq!(lvl, Lvl(1));
                assert_eq!(crate::subst::lookup_sub(&sub, Lvl(1)), num(&def, 0));
            }
            other => panic!("expected Succ, got {other:?}"),
        }
    }

    #[test]
    fn distinct_constructors_are_contradictory() {
        let def = nat_def();
        assert!(matches!(
            unify(Lvl(1), &[(num(&def, 0), num(&def, 1))]),
            UnifyRes::Abs
        ));
        assert!(matches!(
            unify(Lvl(1), &[(num(&def, 1), num(&def, 0))]),
            UnifyRes::Abs
        ));
    }

    #[test]
    fn constructors_are_injective() {
        // `suc x = suc zero` reduces to `x = zero`
        let def = nat_def();
        let suc_x = frc(&Val::ne(Ne::Ctor(def.clone(), 1, vec![Val::vvar(Lvl(1))])));
        let suc_zero = num(&def, 1);
        let r = unify(Lvl(2), &[(suc_x, suc_zero)]);
        match r {
            UnifyRes::Succ { sub, .. } => {
                assert_eq!(crate::subst::lookup_sub(&sub, Lvl(1)), num(&def, 0))
            }
            other => panic!("expected Succ, got {other:?}"),
        }
    }

    #[test]
    fn refl_unifies_only_with_refl() {
        assert!(matches!(
            unify1(Lvl(1), &Val::Refl, &Val::Refl),
            UnifyRes::Succ { .. }
        ));
        assert!(matches!(
            unify1(Lvl(1), &Val::Refl, &Val::vvar(Lvl(0))),
            UnifyRes::Succ { .. }
        ));
    }

    #[test]
    fn stuck_equations_are_not_sure() {
        // `elim Nat .. x` vs `elim Nat .. y` for distinct variables cannot be
        // decided by first-order unification.
        let def = nat_def();
        let stuck_elim = |x: Lvl| {
            Val::ne(Ne::Elim(
                def.clone(),
                vec![Val::U, num(&def, 0), suc_case(&def), Val::vvar(x)],
            ))
        };
        assert!(matches!(
            unify1(Lvl(2), &stuck_elim(Lvl(1)), &stuck_elim(Lvl(0))),
            UnifyRes::Idk
        ));
    }

    #[test]
    fn convertible_equations_succeed() {
        // `elim Nat .. zero` reduces to `zero`
        let def = nat_def();
        let a = Val::ne(Ne::Elim(
            def.clone(),
            vec![Val::U, num(&def, 0), suc_case(&def), num(&def, 0)],
        ));
        assert!(matches!(
            unify1(Lvl(0), &a, &num(&def, 0)),
            UnifyRes::Succ { .. }
        ));
    }

    #[test]
    fn unify_threads_the_substitution_through_the_equations() {
        // [(x, zero), (y, suc x)]  ==>  y := suc zero
        let def = nat_def();
        let r = unify(
            Lvl(2),
            &[
                (Val::vvar(Lvl(1)), num(&def, 0)),
                (
                    Val::vvar(Lvl(0)),
                    frc(&Val::ne(Ne::Ctor(def.clone(), 1, vec![Val::vvar(Lvl(1))]))),
                ),
            ],
        );
        match r {
            UnifyRes::Succ { lvl, sub } => {
                assert_eq!(lvl, Lvl(0));
                let y = crate::subst::lookup_sub(&sub, Lvl(0));
                assert!(conv(Lvl(0), &frc(&y), &num(&def, 1)));
            }
            other => panic!("expected Succ, got {other:?}"),
        }
    }

    #[test]
    fn fmt_substitutes_placeholders_in_order() {
        assert_eq!(fmt("{} and {}", &["a", "b"]), "a and b");
        assert_eq!(fmt("no placeholders", &["x"]), "no placeholders");
    }

    /// Build the type `(x : Id U zero (suc zero)) -> U` for pattern-matching
    /// tests.  The indices are arbitrary; only their unifiability matters.
    fn absurd_pi() -> Val {
        let def = nat_def();
        let id = Val::id(Val::U, num(&def, 0), num(&def, 1));
        Val::pi("_", id, Closure::new(Env::empty(), Tm::U))
    }

    #[test]
    fn absurd_pattern_is_allowed_when_the_equation_is_contradictory() {
        let ctx = crate::context::Ctx::empty(crate::syntax::Pos::initial("<test>"));
        let ok = check_pm(&ctx, &[Pn::Abs], &Raw::Bot, &absurd_pi());
        assert!(ok.is_ok(), "absurd pattern should check: {ok:?}");
    }

    #[test]
    fn absurd_pattern_rejects_a_right_hand_side() {
        // Unreachable from the parser (an absurd lambda always ends in `Bot`),
        // but part of the checker's contract.
        let ctx = crate::context::Ctx::empty(crate::syntax::Pos::initial("<test>"));
        let err = check_pm(&ctx, &[Pn::Abs], &Raw::U, &absurd_pi());
        match err {
            Err(e) => assert!(e.msg.contains("absurd pattern"), "unexpected: {}", e.msg),
            Ok(t) => panic!("expected an error, got {t:?}"),
        }
    }

    #[test]
    fn absurd_pattern_is_rejected_on_a_reflexivity_equation() {
        let ctx = crate::context::Ctx::empty(crate::syntax::Pos::initial("<test>"));
        let def = nat_def();
        let id = Val::id(Val::U, num(&def, 0), num(&def, 0));
        let ty = Val::pi("_", id, Closure::new(Env::empty(), Tm::U));
        match check_pm(&ctx, &[Pn::Abs], &Raw::Bot, &ty) {
            Err(e) => assert!(
                e.msg.contains("should try pattern 'refl'"),
                "unexpected: {}",
                e.msg
            ),
            Ok(t) => panic!("expected an error, got {t:?}"),
        }
    }

    // -- the generated eliminator types -------------------------------------

    #[test]
    fn elim_ty_of_nat() {
        assert_eq!(
            crate::pretty::show_tm(&elim_ty(&nat_def())),
            "(P : Nat → U) → P zero → ((n : Nat)(n' : P n) → P (suc n)) → (x : Nat) → P x"
        );
    }

    #[test]
    fn elim_ty_of_list() {
        assert_eq!(
            crate::pretty::show_tm(&elim_ty(&list_def())),
            "(A : U)(P : List A → U) → P (nil A) → ((a : A)(as : List A)(as' : P as) \
             → P (cons A a as)) → (x : List A) → P x"
        );
    }

    #[test]
    fn elim_ty_of_an_empty_type() {
        let def = Rc::new(IndDef {
            name: "Empty".into(),
            params: vec![],
            ctors: vec![],
        });
        assert_eq!(
            crate::pretty::show_tm(&elim_ty(&def)),
            "(P : Empty → U)(x : Empty) → P x"
        );
    }

    // -- closing stored constructor types -----------------------------------

    #[test]
    fn close_data_replaces_the_datatype_with_the_constant() {
        // suc : Pi(n : #0) #1  closes to  Pi(n : Nat) Nat
        let def = nat_def();
        let suc = &def.ctors[1].ty;
        assert_eq!(
            close_data(suc, &def),
            Tm::Pi(
                "n".into(),
                Box::new(Tm::Data(def.clone())),
                Box::new(Tm::Data(def.clone())),
            )
        );
    }
}
