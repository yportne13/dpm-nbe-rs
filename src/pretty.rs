//! Pretty printing of core terms (and hence of quoted values).
//!
//! Direct port of `Pretty.hs`.

use crate::context::Ctx;
use crate::eval::quote_val;
use crate::syntax::{Ix, Name, Pn, Tm, Val};
use std::fmt;

// Printing precedences.
const ATOM: u8 = 3; // U, variable
const APP: u8 = 2; // application
const PI: u8 = 1; // Π
const LET: u8 = 0; // let, lambda

/// Wrap in parentheses if the expression precedence is lower than the
/// enclosing expression precedence.
fn par(p: u8, p2: u8, s: String) -> String {
    if p2 < p {
        format!("({s})")
    } else {
        s
    }
}

fn fresh(ns: &[Name], x: &str) -> Name {
    if x == "_" {
        return "_".to_string();
    }
    let mut y = x.to_string();
    while ns.iter().any(|n| n == &y) {
        y.push('\'');
    }
    y
}

fn push(ns: &[Name], x: &str) -> Vec<Name> {
    let mut v = Vec::with_capacity(ns.len() + 1);
    v.push(x.to_string());
    v.extend(ns.iter().cloned());
    v
}

/// Print a core term at the given precedence, using `names` for its free
/// variables.
pub fn pretty_tm(prec: u8, names: &[Name], tm: &Tm) -> String {
    go(prec, names, tm)
}

fn pi_bind(ns: &[Name], x: &str, a: &Tm) -> String {
    format!("({} : {})", x, go(LET, ns, a))
}

fn go_pi(ns: &[Name], b: &Tm) -> String {
    match b {
        Tm::Pi(x, a, b2) if x == "_" => {
            format!(" → {} → {}", go(APP, ns, a), go(PI, &push(ns, "_"), b2))
        }
        Tm::Pi(x, a, b2) => {
            let x = fresh(ns, x);
            format!("{}{}", pi_bind(ns, &x, a), go_pi(&push(ns, &x), b2))
        }
        _ => format!(" → {}", go(PI, ns, b)),
    }
}

fn go_lam(t: &Tm, ns: &[Name], ps: &[Pn]) -> String {
    match ps.split_first() {
        None => format!(". {}", go(LET, ns, t)),
        Some((Pn::Var(x), rest)) => {
            let x = fresh(ns, x);
            format!("{} {}", x, go_lam(t, &push(ns, &x), rest))
        }
        Some((Pn::Refl, rest)) => format!("refl {}", go_lam(t, ns, rest)),
        Some((Pn::Abs, rest)) => format!("(!) {}", go_lam(t, ns, rest)),
    }
}

fn go(p: u8, ns: &[Name], tm: &Tm) -> String {
    match tm {
        Tm::Var(Ix(x)) => ns.get(*x).cloned().unwrap_or_else(|| format!("#{x}")),

        Tm::App(t, u) => par(p, APP, format!("{} {}", go(APP, ns, t), go(ATOM, ns, u))),

        Tm::U(0) => "U".to_string(),
        Tm::U(l) => format!("U{l}"),

        Tm::Pi(x, a, b) if x == "_" => par(
            p,
            PI,
            format!("{} → {}", go(APP, ns, a), go(PI, &push(ns, "_"), b)),
        ),
        Tm::Pi(x, a, b) => {
            let x = fresh(ns, x);
            let mut s = pi_bind(ns, &x, a);
            s.push_str(&go_pi(&push(ns, &x), b));
            par(p, PI, s)
        }

        Tm::Let(x, a, t, u) => {
            let x = fresh(ns, x);
            par(
                p,
                LET,
                format!(
                    "let {} : {}\n    = {}\n;\n{}",
                    x,
                    go(LET, ns, a),
                    go(LET, ns, t),
                    go(LET, &push(ns, &x), u)
                ),
            )
        }

        Tm::Lam(ps, t) => par(p, LET, format!("λ {}", go_lam(t, ns, ps))),

        Tm::Id(a, x, y) => par(
            p,
            APP,
            format!(
                "Id {} {} {}",
                go(ATOM, ns, a),
                go(ATOM, ns, x),
                go(ATOM, ns, y)
            ),
        ),

        Tm::Refl => "refl".to_string(),
        Tm::Data(def) => def.name.clone(),
        Tm::Ctor(def, i) => def
            .ctors
            .get(*i)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("{}#{}", def.name, i)),
        Tm::Elim(def) => format!("elim {}", def.name),
        Tm::Bot => "⊥".to_string(),
    }
}

/// Print a closed core term.
pub fn show_tm(tm: &Tm) -> String {
    pretty_tm(0, &[], tm)
}

/// Print a semantic value, quoting it first.
pub fn show_val(ctx: &Ctx, v: &Val) -> String {
    pretty_tm(0, &ctx.ty_names(), &quote_val(ctx.lvl, v))
}

impl fmt::Display for Tm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&show_tm(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{Ix, Pn, Tm};
    use crate::testdata::nat_def;

    fn names(ns: &[&str]) -> Vec<Name> {
        ns.iter().map(|s| s.to_string()).collect()
    }

    fn p(ns: &[&str], tm: &Tm) -> String {
        pretty_tm(0, &names(ns), tm)
    }

    fn v(i: usize) -> Box<Tm> {
        Box::new(Tm::Var(Ix(i)))
    }

    fn nat() -> Tm {
        Tm::Data(nat_def())
    }

    fn zero() -> Tm {
        Tm::Ctor(nat_def(), 0)
    }

    fn suc(n: Tm) -> Tm {
        Tm::App(Box::new(Tm::Ctor(nat_def(), 1)), Box::new(n))
    }

    #[test]
    fn prints_atoms() {
        assert_eq!(show_tm(&Tm::U(0)), "U");
        assert_eq!(show_tm(&nat()), "Nat");
        assert_eq!(show_tm(&zero()), "zero");
        assert_eq!(show_tm(&suc(zero())), "suc zero");
        assert_eq!(show_tm(&Tm::Elim(nat_def())), "elim Nat");
        assert_eq!(show_tm(&Tm::Refl), "refl");
        assert_eq!(show_tm(&Tm::Bot), "\u{22a5}");
    }

    #[test]
    fn prints_ctor_applications_with_parentheses() {
        let two = suc(suc(zero()));
        assert_eq!(show_tm(&two), "suc (suc zero)");
        // elim Nat U zero (suc zero): a stuck eliminator is an application
        let elim = Tm::App(
            Box::new(Tm::App(
                Box::new(Tm::App(
                    Box::new(Tm::Elim(nat_def())),
                    Box::new(Tm::U(0)),
                )),
                Box::new(zero()),
            )),
            Box::new(suc(zero())),
        );
        assert_eq!(show_tm(&elim), "elim Nat U zero (suc zero)");
    }

    #[test]
    fn application_binds_tighter_than_lambda_and_pi() {
        // x (x x)
        let tm = Tm::App(v(0), Box::new(Tm::App(v(0), v(0))));
        assert_eq!(p(&["x"], &tm), "x (x x)");
        // \x. x x
        let lam = Tm::Lam(vec![Pn::Var("x".into())], Box::new(Tm::App(v(0), v(0))));
        assert_eq!(p(&[], &lam), "λ x . x x");
        // (f : Nat) -> f f   (ill-typed, but the printer is syntactic)
        let pi = Tm::Pi("f".into(), Box::new(nat()), Box::new(Tm::App(v(0), v(0))));
        assert_eq!(p(&[], &pi), "(f : Nat) → f f");
    }

    #[test]
    fn consecutive_named_binders_share_no_separator() {
        let tm = Tm::Pi(
            "a".into(),
            Box::new(nat()),
            Box::new(Tm::Pi("b".into(), Box::new(nat()), Box::new(nat()))),
        );
        assert_eq!(show_tm(&tm), "(a : Nat)(b : Nat) → Nat");
    }

    #[test]
    fn anonymous_domains_render_as_arrows() {
        let tm = Tm::Pi("_".into(), Box::new(nat()), Box::new(nat()));
        assert_eq!(show_tm(&tm), "Nat → Nat");
    }

    #[test]
    fn names_are_freshened() {
        // \x. (x' : Nat) -> x'   (the body refers to the inner binder)
        let tm = Tm::Lam(
            vec![Pn::Var("x".into())],
            Box::new(Tm::Pi("x".into(), Box::new(nat()), v(0))),
        );
        assert_eq!(show_tm(&tm), "λ x . (x' : Nat) → x'");
    }

    #[test]
    fn prints_lets() {
        let tm = Tm::Let(
            "x".into(),
            Box::new(nat()),
            Box::new(zero()),
            Box::new(Tm::Var(Ix(0))),
        );
        assert_eq!(show_tm(&tm), "let x : Nat\n    = zero\n;\nx");
    }

    #[test]
    fn prints_id_and_patterns() {
        let tm = Tm::Id(Box::new(nat()), Box::new(zero()), Box::new(zero()));
        assert_eq!(show_tm(&tm), "Id Nat zero zero");
        let lam = Tm::Lam(vec![Pn::Refl, Pn::Abs], Box::new(Tm::Bot));
        assert_eq!(show_tm(&lam), "λ refl (!) . ⊥");
    }

    #[test]
    fn missing_names_degrade_gracefully() {
        assert_eq!(show_tm(&Tm::Var(Ix(3))), "#3");
    }

    #[test]
    fn display_matches_show_tm() {
        assert_eq!(format!("{}", zero()), "zero");
    }
}
