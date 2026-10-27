//! Explicit substitutions and the few environment operations they need.
//!
//! Direct port of `Subst.hs`, with substitutions represented as a sequence of
//! layers instead of a single map (see [`crate::syntax::Sub`]).

use crate::eval::subst_val;
use crate::syntax::{Env, Layer, Lvl, Sub, Tm, Val};
use std::collections::BTreeMap;
use std::rc::Rc;

/// Add `by` to every variable of index `>= cutoff` — the term-level counterpart
/// of inserting `by` binders *outside* the term.  Used by the elaboration of
/// inductive declarations, where a datatype is bound between the parameters and
/// the rest of the context (values carry delayed substitutions instead, but
/// constructor types are manipulated as terms).
pub fn lift(t: &Tm, cutoff: usize, by: usize) -> Tm {
    if by == 0 {
        return t.clone();
    }
    match t {
        Tm::Var(i) => {
            if i.0 >= cutoff {
                Tm::Var(crate::syntax::Ix(i.0 + by))
            } else {
                t.clone()
            }
        }
        Tm::U(_) | Tm::Refl | Tm::Bot | Tm::Data(_) | Tm::Ctor(..) | Tm::Elim(_) | Tm::Meta(_) | Tm::FixRef(_) => t.clone(),
        Tm::App(f, a) => Tm::App(Box::new(lift(f, cutoff, by)), Box::new(lift(a, cutoff, by))),
        Tm::Id(a, x, y) => Tm::Id(
            Box::new(lift(a, cutoff, by)),
            Box::new(lift(x, cutoff, by)),
            Box::new(lift(y, cutoff, by)),
        ),
        Tm::Pi(x, a, b) => Tm::Pi(
            x.clone(),
            Box::new(lift(a, cutoff, by)),
            Box::new(lift(b, cutoff + 1, by)),
        ),
        Tm::Lam(ps, body) => Tm::Lam(
            ps.clone(),
            Box::new(lift(body, cutoff + crate::syntax::num_pvars(ps).0, by)),
        ),
        Tm::Let(x, a, v, b) => Tm::Let(
            x.clone(),
            Box::new(lift(a, cutoff, by)),
            Box::new(lift(v, cutoff, by)),
            Box::new(lift(b, cutoff + 1, by)),
        ),
    }
}

/// A substitution with a single layer.
fn leaf(layer: &Rc<Layer>) -> Sub {
    Sub {
        dom: layer.dom,
        cod: layer.cod,
        layers: vec![layer.clone()],
    }
}

/// Apply a substitution to a level.
///
/// The *oldest* layer that mentions the variable wins, and the newer layers are
/// then applied to its value.  That mirrors the original's left-biased
/// `IM.union (IM.map (subst s2) s1) (subs s2)`, where an entry of the argument
/// `s1` shadows an entry of `s2` for the same level.
pub fn lookup_sub(s: &Sub, i: Lvl) -> Val {
    for (idx, layer) in s.layers.iter().enumerate().rev() {
        if let Some(v) = layer.subs.get(&i.0) {
            let mut r = v.clone();
            for newer in s.layers[..idx].iter().rev() {
                r = subst_val(&leaf(newer), &r);
            }
            return r;
        }
    }
    Val::vvar(i)
}

/// `idSub G : Sub G G`
pub fn id_sub(i: Lvl) -> Sub {
    Sub::id(i)
}

/// `[x |-> v] : Sub (G/{x}) G`, where `v : Val (G/{x})`.
pub fn insert_sub(g: Lvl, x: Lvl, v: Val) -> Sub {
    let mut subs = BTreeMap::new();
    subs.insert(x.0, v);
    Sub::new(g.pred(), g, subs)
}

/// Extend an evaluation environment with a single value.
#[inline]
pub fn define(env: &Env, v: Val) -> Env {
    env.push(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arguments_entry_shadows_the_newer_one() {
        // `IM.union (IM.map (subst s2) s1) (subs s2)` is left-biased, so when
        // both substitutions map the same level, the argument's entry wins and
        // the newer substitution is applied to *its* value.
        let s1 = insert_sub(Lvl(1), Lvl(0), Val::Refl);
        let s2 = insert_sub(Lvl(1), Lvl(0), Val::U(0));
        let composed = crate::eval::subst_sub(&s2, &s1);
        assert_eq!(crate::eval::frc(&lookup_sub(&composed, Lvl(0))), Val::Refl);
    }

    #[test]
    fn lookup_sub_is_the_identity_outside_its_domain() {
        let s = insert_sub(Lvl(2), Lvl(1), Val::Refl);
        assert_eq!(lookup_sub(&s, Lvl(1)), Val::Refl);
        assert_eq!(lookup_sub(&s, Lvl(0)), Val::vvar(Lvl(0)));
    }

    #[test]
    fn insert_sub_narrows_the_domain_by_one() {
        let s = insert_sub(Lvl(3), Lvl(2), Val::U(0));
        assert_eq!(s.dom, Lvl(2));
        assert_eq!(s.cod, Lvl(3));
        assert_eq!(s.layers.len(), 1);
        assert_eq!(s.layers[0].subs.len(), 1);
    }

    #[test]
    fn id_sub_is_empty() {
        let s = id_sub(Lvl(4));
        assert_eq!(s.dom, Lvl(4));
        assert_eq!(s.cod, Lvl(4));
        assert!(s.layers.is_empty());
        assert_eq!(lookup_sub(&s, Lvl(2)), Val::vvar(Lvl(2)));
    }

    // -- lift --------------------------------------------------------------

    #[test]
    fn lift_shifts_only_variables_outside_the_kept_prefix() {
        use crate::syntax::{Ix, Pn};
        // In a context [outer, p0, p1], the term
        //   λ q . App(App(Var 3, Var 2), App(Var 1, Var 0))
        // references p1 (Var 2), p0 (Var 1), outer (Var 3) and the lambda's
        // own q (Var 0).  Keeping the two parameters and shifting the rest by
        // one gives outer -> Var 4, p1 -> Var 2, p0 -> Var 1, q -> Var 0.
        let tm = Tm::Lam(
            vec![Pn::Var("q".into())],
            Box::new(Tm::App(
                Box::new(Tm::App(
                    Box::new(Tm::Var(Ix(3))),
                    Box::new(Tm::Var(Ix(2))),
                )),
                Box::new(Tm::App(
                    Box::new(Tm::Var(Ix(1))),
                    Box::new(Tm::Var(Ix(0))),
                )),
            )),
        );
        let lifted = lift(&tm, 2, 1);
        let expected_body = Tm::App(
            Box::new(Tm::App(
                Box::new(Tm::Var(Ix(4))),
                Box::new(Tm::Var(Ix(2))),
            )),
            Box::new(Tm::App(
                Box::new(Tm::Var(Ix(1))),
                Box::new(Tm::Var(Ix(0))),
            )),
        );
        match &lifted {
            Tm::Lam(ps, body) => {
                assert_eq!(ps.len(), 1);
                assert_eq!(**body, expected_body);
            }
            other => panic!("lift must not change the lambda shape: {other:?}"),
        }
        // `by = 0` is the identity.
        assert_eq!(lift(&tm, 0, 0), tm);
    }

    #[test]
    fn lift_descends_past_pi_let_and_pattern_binders() {
        use crate::syntax::{Ix, Pn};
        // Pi(x, Var 1, Let(y, Var 2, Var 1, Var 0)):
        //   - the Pi domain Var 1 is above cutoff 0 -> 2;
        //   - inside the body (cutoff 1): let's type Var 2 -> 3, value Var 1 -> 2,
        //     and the let body (cutoff 2) has Var 0/1 unchanged... Var 0 stays,
        //     Var 1 refers to the let binder -> stays.
        let tm = Tm::Pi(
            "x".into(),
            Box::new(Tm::Var(Ix(1))),
            Box::new(Tm::Let(
                "y".into(),
                Box::new(Tm::Var(Ix(2))),
                Box::new(Tm::Var(Ix(1))),
                Box::new(Tm::Var(Ix(1))),
            )),
        );
        assert_eq!(
            lift(&tm, 0, 1),
            Tm::Pi(
                "x".into(),
                Box::new(Tm::Var(Ix(2))),
                Box::new(Tm::Let(
                    "y".into(),
                    Box::new(Tm::Var(Ix(3))),
                    Box::new(Tm::Var(Ix(2))),
                    Box::new(Tm::Var(Ix(1))),
                )),
            )
        );
        // A refl pattern binds nothing, so it costs no cutoff.
        let lam = Tm::Lam(vec![Pn::Refl], Box::new(Tm::Var(Ix(0))));
        assert_eq!(lift(&lam, 0, 1), Tm::Lam(vec![Pn::Refl], Box::new(Tm::Var(Ix(1)))));
    }
}
