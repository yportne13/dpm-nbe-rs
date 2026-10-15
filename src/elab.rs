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
use crate::eval::{conv, eval, frc, subst_cl, subst_sub, subst_val};
use crate::pretty::show_val;
use crate::subst::{id_sub, insert_sub};
use crate::syntax::*;

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
    let (pair, rest) = match vs.split_first() {
        None => {
            return UnifyRes::Succ {
                lvl: g,
                sub: gamma.clone(),
            }
        }
        Some(x) => x,
    };
    let (v1, v2) = pair;
    match unify1(g, v1, v2) {
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
            sub: insert_sub(g, x, v),
        };
    }
    if let Some(x) = as_var(&v) {
        return UnifyRes::Succ {
            lvl: g.pred(),
            sub: insert_sub(g, x, u),
        };
    }

    match (&u, &v) {
        (Val::Refl, Val::Refl) => UnifyRes::Succ {
            lvl: g,
            sub: id_sub(g),
        },
        (Val::Zero, Val::Zero) => UnifyRes::Succ {
            lvl: g,
            sub: id_sub(g),
        },
        // `suc` is injective.
        (Val::Succ(m), Val::Succ(n)) => unify1(g, m, n),
        (Val::Zero, Val::Succ(_)) | (Val::Succ(_), Val::Zero) => UnifyRes::Abs,
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

        Raw::Nat => Ok((Tm::Nat, Val::U)),
        Raw::Zero => Ok((Tm::Zero, Val::Nat)),
        Raw::Succ(n) => {
            let n = check(ctx, n, &Val::Nat)?;
            Ok((Tm::Succ(Box::new(n)), Val::Nat))
        }
        Raw::Plus(m, n) => {
            let m = check(ctx, m, &Val::Nat)?;
            let n = check(ctx, n, &Val::Nat)?;
            Ok((Tm::Plus(Box::new(m), Box::new(n)), Val::Nat))
        }
        Raw::Bot => panic!("infer: impossible (Bot)"),
    }
}

/// Tiny `printf`-style helper: substitute `{}` placeholders left to right.
fn fmt(template: &str, args: &[&str]) -> String {
    args.iter()
        .fold(template.to_string(), |s, a| s.replacen("{}", a, 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Ne;

    fn plus_val(a: Val, b: Val) -> Val {
        Val::ne(Ne::Plus(a, b))
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
        let r = unify1(Lvl(2), &Val::vvar(Lvl(1)), &Val::Zero);
        match r {
            UnifyRes::Succ { lvl, sub } => {
                assert_eq!(lvl, Lvl(1));
                assert_eq!(crate::subst::lookup_sub(&sub, Lvl(1)), Val::Zero);
            }
            other => panic!("expected Succ, got {other:?}"),
        }
    }

    #[test]
    fn zero_and_succ_are_contradictory() {
        assert!(matches!(
            unify1(Lvl(1), &Val::Zero, &Val::succ(Val::Zero)),
            UnifyRes::Abs
        ));
        assert!(matches!(
            unify1(Lvl(1), &Val::succ(Val::Zero), &Val::Zero),
            UnifyRes::Abs
        ));
    }

    #[test]
    fn suc_is_injective() {
        // `suc x = suc zero` reduces to `x = zero`
        let r = unify1(Lvl(2), &Val::succ(Val::vvar(Lvl(1))), &Val::succ(Val::Zero));
        match r {
            UnifyRes::Succ { sub, .. } => {
                assert_eq!(crate::subst::lookup_sub(&sub, Lvl(1)), Val::Zero)
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
        // `plus x zero` vs `plus y zero` for distinct variables cannot be
        // decided by first-order unification.
        let a = plus_val(Val::vvar(Lvl(1)), Val::Zero);
        let b = plus_val(Val::vvar(Lvl(0)), Val::Zero);
        assert!(matches!(unify1(Lvl(2), &a, &b), UnifyRes::Idk));
    }

    #[test]
    fn convertible_equations_succeed() {
        // `plus zero zero` is convertible with `zero`
        let a = plus_val(Val::Zero, Val::Zero);
        assert!(matches!(
            unify1(Lvl(0), &a, &Val::Zero),
            UnifyRes::Succ { .. }
        ));
    }

    #[test]
    fn unify_threads_the_substitution_through_the_equations() {
        // [(x, zero), (y, suc x)]  ==>  y := suc zero
        let r = unify(
            Lvl(2),
            &[
                (Val::vvar(Lvl(1)), Val::Zero),
                (Val::vvar(Lvl(0)), Val::succ(Val::vvar(Lvl(1)))),
            ],
        );
        match r {
            UnifyRes::Succ { lvl, sub } => {
                assert_eq!(lvl, Lvl(0));
                let y = crate::subst::lookup_sub(&sub, Lvl(0));
                assert!(conv(Lvl(0), &frc(&y), &Val::succ(Val::Zero)));
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
        let id = Val::id(Val::U, Val::Zero, Val::succ(Val::Zero));
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
        let err = check_pm(&ctx, &[Pn::Abs], &Raw::Nat, &absurd_pi());
        match err {
            Err(e) => assert!(e.msg.contains("absurd pattern"), "unexpected: {}", e.msg),
            Ok(t) => panic!("expected an error, got {t:?}"),
        }
    }

    #[test]
    fn absurd_pattern_is_rejected_on_a_reflexivity_equation() {
        let ctx = crate::context::Ctx::empty(crate::syntax::Pos::initial("<test>"));
        let id = Val::id(Val::U, Val::Zero, Val::Zero);
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
}
