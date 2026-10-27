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

/// Run `f`, wrapping its output in parentheses when the inner precedence
/// `p2` is lower than the enclosing expression precedence `p`.
///
/// Everything appends into one caller-owned `String`: the printer used to
/// build a fresh `String` per node and concatenate, which is quadratic on
/// deep application spines (a ten-thousand-successor numeral copied its
/// prefix once per level).
fn with_par(p: u8, p2: u8, out: &mut String, f: impl FnOnce(&mut String)) {
    if p2 < p {
        out.push('(');
        f(out);
        out.push(')');
    } else {
        f(out);
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
    let mut out = String::new();
    go(prec, names, tm, &mut out);
    out
}

fn pi_bind(ns: &[Name], x: &str, a: &Tm, out: &mut String) {
    out.push('(');
    out.push_str(x);
    out.push_str(" : ");
    go(LET, ns, a, out);
    out.push(')');
}

fn go_pi(ns: &[Name], b: &Tm, out: &mut String) {
    match b {
        Tm::Pi(x, a, b2) if x == "_" => {
            out.push_str(" → ");
            go(APP, ns, a, out);
            out.push_str(" → ");
            go(PI, &push(ns, "_"), b2, out);
        }
        Tm::Pi(x, a, b2) => {
            let x = fresh(ns, x);
            pi_bind(ns, &x, a, out);
            go_pi(&push(ns, &x), b2, out);
        }
        _ => {
            out.push_str(" → ");
            go(PI, ns, b, out);
        }
    }
}

fn go_lam(t: &Tm, ns: &[Name], ps: &[Pn], out: &mut String) {
    match ps.split_first() {
        None => {
            out.push_str(". ");
            go(LET, ns, t, out);
        }
        Some((Pn::Var(x), rest)) => {
            let x = fresh(ns, x);
            out.push_str(&x);
            out.push(' ');
            go_lam(t, &push(ns, &x), rest, out);
        }
        Some((Pn::Refl, rest)) => {
            out.push_str("refl ");
            go_lam(t, ns, rest, out);
        }
        Some((Pn::Abs, rest)) => {
            out.push_str("(!) ");
            go_lam(t, ns, rest, out);
        }
    }
}

fn go(p: u8, ns: &[Name], tm: &Tm, out: &mut String) {
    match tm {
        Tm::Var(Ix(x)) => match ns.get(*x) {
            Some(n) => out.push_str(n),
            None => out.push_str(&format!("#{x}")),
        },

        Tm::App(t, u) => with_par(p, APP, out, |out| {
            go(APP, ns, t, out);
            out.push(' ');
            go(ATOM, ns, u, out);
        }),

        Tm::U(0) => out.push_str("U"),
        Tm::U(l) => out.push_str(&format!("U{l}")),

        Tm::Pi(x, a, b) if x == "_" => with_par(p, PI, out, |out| {
            go(APP, ns, a, out);
            out.push_str(" → ");
            go(PI, &push(ns, "_"), b, out);
        }),
        Tm::Pi(x, a, b) => {
            let x = fresh(ns, x);
            with_par(p, PI, out, |out| {
                pi_bind(ns, &x, a, out);
                go_pi(&push(ns, &x), b, out);
            })
        }

        Tm::Let(x, a, t, u) => {
            let x = fresh(ns, x);
            with_par(p, LET, out, |out| {
                out.push_str("let ");
                out.push_str(&x);
                out.push_str(" : ");
                go(LET, ns, a, out);
                out.push_str("\n    = ");
                go(LET, ns, t, out);
                out.push_str("\n;\n");
                go(LET, &push(ns, &x), u, out);
            })
        }

        Tm::Lam(ps, t) => with_par(p, LET, out, |out| {
            out.push_str("λ ");
            go_lam(t, ns, ps, out);
        }),

        Tm::Id(a, x, y) => with_par(p, APP, out, |out| {
            out.push_str("Id ");
            go(ATOM, ns, a, out);
            out.push(' ');
            go(ATOM, ns, x, out);
            out.push(' ');
            go(ATOM, ns, y, out);
        }),

        Tm::Refl => out.push_str("refl"),
        Tm::Data(def) => out.push_str(&def.name),
        Tm::Ctor(def, i) => match def.ctors.get(*i) {
            Some(c) => out.push_str(&c.name),
            None => out.push_str(&format!("{}#{}", def.name, i)),
        },
        Tm::Elim(def) => {
            out.push_str("elim ");
            out.push_str(&def.name);
        }
        Tm::Meta(m) => out.push_str(&format!("?{m}")),
        Tm::FixRef(id) => out.push_str(&crate::fix::name_of(*id)),
        Tm::Bot => out.push_str("⊥"),
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
