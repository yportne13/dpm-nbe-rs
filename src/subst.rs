//! Explicit substitutions and the few environment operations they need.
//!
//! Direct port of `Subst.hs`, with substitutions represented as a sequence of
//! layers instead of a single map (see [`crate::syntax::Sub`]).

use crate::eval::subst_val;
use crate::syntax::{Env, Layer, Lvl, Sub, Val};
use std::collections::BTreeMap;
use std::rc::Rc;

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
        let s1 = insert_sub(Lvl(1), Lvl(0), Val::Zero);
        let s2 = insert_sub(Lvl(1), Lvl(0), Val::succ(Val::Zero));
        let composed = crate::eval::subst_sub(&s2, &s1);
        assert_eq!(crate::eval::frc(&lookup_sub(&composed, Lvl(0))), Val::Zero);
    }

    #[test]
    fn lookup_sub_is_the_identity_outside_its_domain() {
        let s = insert_sub(Lvl(2), Lvl(1), Val::Zero);
        assert_eq!(lookup_sub(&s, Lvl(1)), Val::Zero);
        assert_eq!(lookup_sub(&s, Lvl(0)), Val::vvar(Lvl(0)));
    }

    #[test]
    fn insert_sub_narrows_the_domain_by_one() {
        let s = insert_sub(Lvl(3), Lvl(2), Val::U);
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
}
