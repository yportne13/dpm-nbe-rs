//! `prune`: the declarations the goal cannot reach, removed.
//!
//! A program is a linear chain of top-level declarations (`let`, `data`,
//! `struct`), each scoping over the rest of the chain, ending in the goal
//! expression.  Pruning keeps exactly the declarations the goal can still
//! name: those it mentions, those *they* mention, and so on.
//!
//! Resolution is lexical, innermost declaration first, so pruning never
//! rebinds a name: a use is resolved by the innermost declaration that binds
//! it, and that declaration is kept whenever the use is.  Dropped
//! declarations bind nothing any kept part resolves against -- if one did, it
//! would have been the resolver and been kept instead.  The pruned program
//! therefore elaborates exactly like the original.
//!
//! Pruning parses but never typechecks, so a broken declaration survives as
//! long as nothing reachable names it -- which is the point.

use crate::syntax::{Name, Pn, Raw};
use crate::{parse, Error};
use std::collections::HashSet;

/// Prune a program from `src`, reporting positions as `file`.
pub fn prune_named(src: &str, file: &str) -> Result<String, Error> {
    let raw = parse::parse_named(src, file)?;
    Ok(prune_program(&raw))
}

/// Prune a program, reporting positions as `<stdin>`.
pub fn prune(src: &str) -> Result<String, Error> {
    prune_named(src, "<stdin>")
}

/// Prune a parsed program and print it back as surface syntax.
pub fn prune_program(raw: &Raw) -> String {
    let (decls, goal) = split(raw);
    let keep = reachable(&decls, goal);

    let mut out = String::new();
    for (i, d) in decls.iter().enumerate() {
        if keep[i] {
            print_decl(d, &mut out);
            // The next line (or the goal) parses as this declaration's body.
            out.push('\n');
        }
    }
    print_raw(LET, goal, &mut out);
    out
}

// ---------------------------------------------------------------------------
// The declaration chain
// ---------------------------------------------------------------------------

/// One top-level declaration, split off the chain, still borrowing the tree.
enum Decl<'a> {
    /// `let x : ty = val; ...`
    Let(&'a Name, &'a Raw, &'a Raw),
    /// `data d params : ty where ctors; ...`
    Data(&'a Name, &'a [(Name, Raw)], &'a Raw, &'a [(Name, Raw)]),
    /// `struct d params : ty where fields; ...`
    Struct(&'a Name, &'a [(Name, Raw)], &'a Raw, &'a [(Name, Raw)]),
}

impl Decl<'_> {
    /// The names the declaration binds for the rest of the program.  A
    /// `let _` binds nothing; parameters stay local to their declaration.
    /// A struct additionally binds the constructor and projections its
    /// sugar generates (`mkS`, one `let` per field -- see `elab::elab_struct`).
    fn binds(&self) -> Vec<Name> {
        match self {
            Decl::Let(x, _, _) if x.as_str() == "_" => Vec::new(),
            Decl::Let(x, _, _) => vec![x.to_string()],
            Decl::Data(d, _, _, cs) => {
                let mut v = vec![d.to_string()];
                v.extend(cs.iter().map(|(c, _)| c.to_string()));
                v
            }
            Decl::Struct(s, _, _, fs) => {
                let mut v = vec![s.to_string(), format!("mk{s}")];
                v.extend(fs.iter().map(|(f, _)| f.to_string()));
                v
            }
        }
    }

    /// The outer names the declaration's own components use, i.e. its
    /// dependencies on earlier declarations.  The declaration's own binders
    /// (parameters, the type itself, its constructors/fields) shadow during
    /// the traversal, mirroring the elaborator's scoping: parameters see the
    /// previous ones, the type after them sees no self-reference, and the
    /// constructor/field types see all of the above.
    fn needs(&self) -> HashSet<Name> {
        let mut out = HashSet::new();
        match self {
            Decl::Let(_, ty, val) => {
                free_under(ty, &[], &mut out);
                free_under(val, &[], &mut out);
            }
            Decl::Data(d, params, ty, items) | Decl::Struct(d, params, ty, items) => {
                let mut bound: Vec<Name> = Vec::new();
                for (x, a) in *params {
                    free_under(a, &bound, &mut out);
                    bound.push(x.to_string());
                }
                free_under(ty, &bound, &mut out);
                bound.push(d.to_string());
                bound.extend(items.iter().map(|(c, _)| c.to_string()));
                for (_, t) in *items {
                    free_under(t, &bound, &mut out);
                }
            }
        }
        out
    }
}

/// Split a program into its top-level declarations and the goal.  Position
/// wrappers are transparent.
fn split<'a>(raw: &'a Raw) -> (Vec<Decl<'a>>, &'a Raw) {
    let mut decls = Vec::new();
    let mut r = raw;
    loop {
        let t = match r {
            Raw::SrcPos(_, t) => t,
            _ => r,
        };
        r = match t {
            Raw::Let(x, ty, val, body) => {
                decls.push(Decl::Let(x, ty, val));
                body
            }
            Raw::Data(d, ps, ty, cs, body) => {
                decls.push(Decl::Data(d, ps, ty, cs));
                body
            }
            Raw::Struct(d, ps, ty, fs, body) => {
                decls.push(Decl::Struct(d, ps, ty, fs));
                body
            }
            _ => return (decls, t),
        };
    }
}

// ---------------------------------------------------------------------------
// Free names
// ---------------------------------------------------------------------------

/// The free names of `r`, given the binders already in scope (`pre`).
fn free_under(r: &Raw, pre: &[Name], out: &mut HashSet<Name>) {
    let mut bound = pre.to_vec();
    free(r, &mut bound, out);
}

/// Collect free names, tracking every binder the term itself introduces.
/// Nested declarations bind their names over their bodies, exactly as at the
/// top level.  A projection's field (`x.f`) names a field of `x`'s type, not
/// a binding in scope, so only `x` is collected.
fn free(r: &Raw, bound: &mut Vec<Name>, out: &mut HashSet<Name>) {
    match r {
        Raw::SrcPos(_, t) => free(t, bound, out),
        Raw::Var(x) => {
            if !bound.contains(x) {
                out.insert(x.clone());
            }
        }
        Raw::App(a, b) => {
            free(a, bound, out);
            free(b, bound, out);
        }
        Raw::U(_) | Raw::Refl | Raw::Bot => {}
        Raw::Let(x, a, t, u) => {
            free(a, bound, out);
            free(t, bound, out);
            let n0 = bound.len();
            bound.push(x.clone());
            free(u, bound, out);
            bound.truncate(n0);
        }
        Raw::Pi(x, a, b) => {
            free(a, bound, out);
            let n0 = bound.len();
            bound.push(x.clone());
            free(b, bound, out);
            bound.truncate(n0);
        }
        Raw::Lam(ps, t) => {
            let n0 = bound.len();
            for p in ps {
                if let Pn::Var(x) = p {
                    bound.push(x.clone());
                }
            }
            free(t, bound, out);
            bound.truncate(n0);
        }
        Raw::Id(a, x, y) => {
            free(a, bound, out);
            free(x, bound, out);
            free(y, bound, out);
        }
        Raw::Data(d, params, ty, cs, body) | Raw::Struct(d, params, ty, cs, body) => {
            let n0 = bound.len();
            for (x, a) in params {
                free(a, bound, out);
                bound.push(x.clone());
            }
            free(ty, bound, out);
            bound.push(d.clone());
            bound.extend(cs.iter().map(|(c, _)| c.clone()));
            for (_, c) in cs {
                free(c, bound, out);
            }
            free(body, bound, out);
            bound.truncate(n0);
        }
        Raw::Proj(x, _) => free(x, bound, out),
        Raw::Elim(a) => free(a, bound, out),
    }
}

// ---------------------------------------------------------------------------
// Reachability
// ---------------------------------------------------------------------------

/// Which declarations to keep: a declaration is reachable when the goal (or a
/// reachable declaration) resolves a name to it.
fn reachable(decls: &[Decl<'_>], goal: &Raw) -> Vec<bool> {
    let binds: Vec<Vec<Name>> = decls.iter().map(|d| d.binds()).collect();
    let mut keep = vec![false; decls.len()];

    // The innermost declaration before `upto` binding `x`; the goal sits
    // after them all.  Names nothing binds (they would be unbound variables
    // at elaboration) resolve to nothing and are dropped.
    let resolve = |x: &Name, upto: usize| -> Option<usize> {
        (0..upto).rev().find(|&j| binds[j].iter().any(|y| y == x))
    };

    let mut goal_free = HashSet::new();
    free_under(goal, &[], &mut goal_free);
    let mut queue: Vec<usize> = Vec::new();
    for x in &goal_free {
        if let Some(j) = resolve(x, binds.len()) {
            if !keep[j] {
                keep[j] = true;
                queue.push(j);
            }
        }
    }
    while let Some(j) = queue.pop() {
        for x in decls[j].needs() {
            if let Some(k) = resolve(&x, j) {
                if !keep[k] {
                    keep[k] = true;
                    queue.push(k);
                }
            }
        }
    }
    keep
}

// ---------------------------------------------------------------------------
// Surface printing
// ---------------------------------------------------------------------------

// Printing precedences, mirroring the grammar (pretty.rs has the same ladder
// for core terms): atoms are variables, universes, `refl`, `elim D`,
// projections and parenthesised terms; application and `Id` bind tighter than
// the arrow; `let`, lambda and declarations bind loosest.
const ATOM: u8 = 3;
const APP: u8 = 2;
const PI: u8 = 1;
const LET: u8 = 0;

/// Run `f`, wrapping its output in parentheses when the inner precedence
/// `p2` is lower than the enclosing expression precedence `p`.
fn with_par(p: u8, p2: u8, out: &mut String, f: impl FnOnce(&mut String)) {
    if p2 < p {
        out.push('(');
        f(out);
        out.push(')');
    } else {
        f(out);
    }
}

/// Print a kept declaration as one line of source.  A let ends in `;`, a
/// data/struct in its last constructor's `;` (or in `where`, when it has
/// none) -- either way the next line parses as the declaration's body, so
/// the chain re-parses as it printed.
fn print_decl(d: &Decl<'_>, out: &mut String) {
    match d {
        Decl::Let(x, ty, val) => {
            out.push_str("let ");
            out.push_str(x);
            out.push_str(" : ");
            print_raw(LET, ty, out);
            out.push_str(" = ");
            print_raw(LET, val, out);
            out.push(';');
        }
        Decl::Data(n, ps, ty, cs) => print_decl_head("data", n, ps, ty, cs, out),
        Decl::Struct(n, ps, ty, fs) => print_decl_head("struct", n, ps, ty, fs, out),
    }
}

/// The `data`/`struct` part up to and including the last `;` of the
/// constructor/field list; the body is what follows.
fn print_decl_head(
    kw: &str,
    name: &Name,
    params: &[(Name, Raw)],
    ty: &Raw,
    items: &[(Name, Raw)],
    out: &mut String,
) {
    out.push_str(kw);
    out.push(' ');
    out.push_str(name);
    for (x, a) in params {
        out.push_str(" (");
        out.push_str(x);
        out.push_str(" : ");
        print_raw(LET, a, out);
        out.push(')');
    }
    out.push_str(" : ");
    print_raw(LET, ty, out);
    out.push_str(" where");
    for (c, t) in items {
        out.push(' ');
        out.push_str(c);
        out.push_str(" : ");
        print_raw(LET, t, out);
        out.push(';');
    }
}

/// Print a surface term at the given precedence.
fn print_raw(p: u8, r: &Raw, out: &mut String) {
    match r {
        Raw::SrcPos(_, t) => print_raw(p, t, out),
        Raw::Var(x) => out.push_str(x),
        Raw::U(0) => out.push_str("U"),
        Raw::U(l) => out.push_str(&format!("U{l}")),
        Raw::Refl => out.push_str("refl"),
        Raw::Bot => out.push_str("(!)"),
        Raw::App(f, x) => with_par(p, APP, out, |o| {
            print_raw(APP, f, o);
            o.push(' ');
            print_raw(ATOM, x, o);
        }),
        Raw::Id(a, x, y) => with_par(p, APP, out, |o| {
            o.push_str("Id ");
            print_raw(ATOM, a, o);
            o.push(' ');
            print_raw(ATOM, x, o);
            o.push(' ');
            print_raw(ATOM, y, o);
        }),
        Raw::Elim(a) => with_par(p, ATOM, out, |o| {
            o.push_str("elim ");
            print_raw(ATOM, a, o);
        }),
        Raw::Proj(x, f) => with_par(p, ATOM, out, |o| {
            print_raw(ATOM, x, o);
            o.push('.');
            o.push_str(f);
        }),
        Raw::Pi(x, a, b) if x == "_" => with_par(p, PI, out, |o| {
            print_raw(APP, a, o);
            o.push_str(" -> ");
            print_raw(PI, b, o);
        }),
        Raw::Pi(x, a, b) => with_par(p, PI, out, |o| {
            o.push('(');
            o.push_str(x);
            o.push_str(" : ");
            print_raw(LET, a, o);
            o.push_str(") -> ");
            print_raw(PI, b, o);
        }),
        Raw::Lam(ps, t) => with_par(p, LET, out, |o| {
            o.push_str("λ ");
            for (i, q) in ps.iter().enumerate() {
                if i > 0 {
                    o.push(' ');
                }
                match q {
                    Pn::Var(x) => o.push_str(x),
                    Pn::Refl => o.push_str("refl"),
                    Pn::Abs => o.push_str("(!)"),
                }
            }
            // An absurd lambda has no body at all: the parser builds
            // `\ p ... (!)` as patterns plus `Raw::Bot`, and prints back the
            // same way round.
            if !matches!(&**t, Raw::Bot) || !matches!(ps.last(), Some(Pn::Abs)) {
                o.push_str(" . ");
                print_raw(LET, t, o);
            }
        }),
        Raw::Let(x, a, t, u) => with_par(p, LET, out, |o| {
            o.push_str("let ");
            o.push_str(x);
            o.push_str(" : ");
            print_raw(LET, a, o);
            o.push_str(" = ");
            print_raw(LET, t, o);
            o.push_str("; ");
            print_raw(LET, u, o);
        }),
        Raw::Data(d, ps, ty, cs, body) => with_par(p, LET, out, |o| {
            print_decl_head("data", d, ps, ty, cs, o);
            o.push(' ');
            print_raw(LET, body, o);
        }),
        Raw::Struct(d, ps, ty, fs, body) => with_par(p, LET, out, |o| {
            print_decl_head("struct", d, ps, ty, fs, o);
            o.push(' ');
            print_raw(LET, body, o);
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run, Mode};

    const NAT: &str = "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;";
    const NAT_PLUS: &str = "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat; \
let plus : Nat -> Nat -> Nat = \
λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;";

    /// Prune and return the output, panicking on parse errors.
    fn pruned(src: &str) -> String {
        prune(src).unwrap_or_else(|e| panic!("{src}\n=> {}", e.msg))
    }

    #[test]
    fn drops_what_the_goal_cannot_name() {
        let src = format!(
            "{NAT} \
data Unused : U where mk : Unused; \
let dead : Nat = suc (suc zero); \
let two : Nat = suc (suc zero); \
suc two"
        );
        assert_eq!(
            pruned(&src),
            "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;\n\
             let two : Nat = suc (suc zero);\n\
             suc two"
        );
    }

    #[test]
    fn keeps_the_transitive_dependencies() {
        // `two` needs `suc` and `zero` (Nat); `dead` needs Nat too but is
        // itself unneeded -- Nat stays because of `two`, not `dead`.
        let src = format!(
            "{NAT} \
let dead : Nat = zero; \
let two : Nat = suc (suc zero); \
two"
        );
        let out = pruned(&src);
        assert!(out.contains("let two"));
        assert!(!out.contains("dead"));
        assert!(out.contains("data Nat"));
    }

    #[test]
    fn the_innermost_binder_resolves_a_use() {
        // The second `zero` shadows the constructor; the goal must resolve to
        // the let, and the let's own value (where `zero` still means the
        // constructor) must pull in the declaration.
        let src = format!("{NAT} let zero : Nat = suc zero; zero");
        assert_eq!(
            pruned(&src),
            "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;\n\
             let zero : Nat = suc zero;\n\
             zero"
        );
        // A use *before* the shadowing declaration (here, inside `f`) still
        // resolves to the constructor, so the shadow is unreachable and goes.
        let src = format!("{NAT} let f : Nat -> Nat = λ n . zero; let zero : Nat = suc zero; f");
        let out = pruned(&src);
        assert!(out.contains("let f"));
        assert!(out.contains("data Nat"));
        assert!(!out.contains("let zero"));
    }

    #[test]
    fn dropped_declarations_never_rebind() {
        // If the goal resolves `x` to the second let, the first `x` is kept
        // only because the second let's *value* needs it.
        let src = format!("{NAT} let x : Nat = zero; let x : Nat = suc x; x");
        assert_eq!(
            pruned(&src),
            "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;\n\
             let x : Nat = zero;\n\
             let x : Nat = suc x;\n\
             x"
        );
    }

    #[test]
    fn data_and_struct_bind_their_generated_names() {
        // A struct binds its type, its `mkS` and every projection; using any
        // of them keeps the declaration (and `Nat`, which the field needs).
        let src = format!(
            "{NAT} \
struct Pair (A : U) (B : U) : U where fst : A; snd : B; \
data Bool : U where true : Bool; false : Bool; \
mkPair Nat Nat zero (suc zero)"
        );
        let out = pruned(&src);
        assert!(out.contains("struct Pair"));
        assert!(!out.contains("data Bool"));
        // the generated projection is an ordinary binding: `fst` alone keeps
        // the struct, applied through a local pair
        let src = format!(
            "{NAT} \
struct Pair (A : U) (B : U) : U where fst : A; snd : B; \
let p : Pair Nat Nat = mkPair Nat Nat (suc zero) zero; \
fst Nat Nat p"
        );
        let out = pruned(&src);
        assert!(out.contains("struct Pair"));
        assert!(out.contains("let p"));
    }

    #[test]
    fn elim_references_go_through_the_type_name() {
        let src = format!(
            "{NAT} \
data Bool : U where true : Bool; false : Bool; \
elim Bool (λ _ . Nat) zero zero true"
        );
        let out = pruned(&src);
        assert!(out.contains("data Bool"));
        assert!(out.contains("data Nat"));
    }

    #[test]
    fn let_underscore_binds_nothing() {
        let src = format!("{NAT} let _ : Nat = zero; suc zero");
        assert_eq!(
            pruned(&src),
            "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;\nsuc zero"
        );
    }

    #[test]
    fn the_empty_decl_form_prints_without_constructors() {
        let src = "data Empty : U where data Unused : U where mk : Unused; Empty";
        assert_eq!(pruned(src), "data Empty : U where\nEmpty");
    }

    #[test]
    fn a_program_of_one_expression_prints_as_itself() {
        assert_eq!(pruned("U"), "U");
        assert_eq!(pruned("(A : U) -> A -> A"), "(A : U) -> A -> A");
        assert_eq!(pruned("\\ x . x"), "λ x . x");
        assert_eq!(pruned("λ A (!)"), "λ A (!)");
    }

    #[test]
    fn broken_unreachable_declarations_are_dropped_not_reported() {
        // Pruning never typechecks: an ill-typed `dead` disappears, and an
        // ill-typed *reachable* declaration is printed as-is.
        let src = format!("{NAT} let dead : Nat = U; suc zero");
        assert!(!pruned(&src).contains("dead"));
        let src = format!("{NAT} let bad : Nat = U; bad");
        assert!(pruned(&src).contains("let bad : Nat = U;"));
    }

    #[test]
    fn parse_errors_are_still_reported() {
        let e = prune("let x : Nat = ").unwrap_err();
        assert!(e.msg.contains("expected") || e.msg.contains("unexpected"));
    }

    #[test]
    fn pruned_programs_elaborate_identically() {
        // The property the whole module is after: pruning changes neither the
        // normal form nor the type of a well-typed program.
        let readme = "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat; \
let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = \
λ A a b c refl refl . refl; \
let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) = \
λ A B f a b refl . refl; \
let plus : Nat -> Nat -> Nat = \
λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m; \
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = \
λ m n refl refl . refl; \
let C : (A : U) -> Id Nat (suc zero) zero -> A = \
λ A (!); \
            plus (suc zero) (suc (suc zero))";
        let progs = [
            readme.to_string(),
            format!("{NAT_PLUS} plus (suc zero) (suc zero)"),
            format!("{NAT} {PAIR_PROG}"), // a struct program, see below
        ];
        for src in &progs {
            let src = src.as_str();
            let pruned_src = pruned(src);
            for mode in [Mode::Nf, Mode::Type] {
                assert_eq!(
                    run(src, mode).unwrap_or_else(|e| panic!("{src}\n=> {}", e.msg)),
                    run(&pruned_src, mode)
                        .unwrap_or_else(|e| panic!("pruned: {pruned_src}\n=> {}", e.msg)),
                    "pruning changed the {mode:?} output of\n{src}"
                );
            }
        }
    }

    const PAIR_DECLS: &str = "struct Pair (A : U) (B : U) : U where fst : A; snd : B; \
let swap : (A : U) (B : U) -> Pair A B -> Pair B A = \
λ A B p . mkPair B A (p.snd) (p.fst);";

    const PAIR_PROG: &str = "struct Pair (A : U) (B : U) : U where fst : A; snd : B; \
let swap : (A : U) (B : U) -> Pair A B -> Pair B A = \
λ A B p . mkPair B A (p.snd) (p.fst); \
(swap Nat Nat (mkPair Nat Nat zero (suc zero))).snd";

    #[test]
    fn pruning_is_idempotent_and_prints_back() {
        let src = format!(
            "{NAT_PLUS} {PAIR_DECLS} \
data Unused : U where mk : Unused; \
(swap Nat Nat (mkPair Nat Nat zero (suc zero))).snd"
        );
        let once = pruned(&src);
        let twice = pruned(&once);
        assert_eq!(once, twice);
        // the printed form re-parses (it must: `twice` is its own prune)
        assert!(!once.contains("Unused"));
        assert!(once.contains("struct Pair"));
    }

    #[test]
    fn run_prune_reports_positions_from_the_named_file() {
        let e = prune_named("(", "prog.dpm").unwrap_err();
        assert_eq!(e.pos.file.to_string(), "prog.dpm");
    }
}
