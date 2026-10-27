//! The typeclass machinery: the class registry, the instance table, and
//! instance resolution.
//!
//! A `class` declaration is struct sugar (NOTES.md deviation 16) plus a
//! registry entry: its single-constructor type `C ps` and the parameters.
//! Its projections then have `(q : C ps)` domains, and that shape is what
//! drives everything else:
//!
//! * **Automatic insertion** (`elab::insert_instances`): elaborating an
//!   application skips the function type's class-application domains,
//!   filling each with a resolved instance, so `eq zero zero` works
//!   without passing the dictionary.  The explicit escape hatch is the
//!   field projection `d.eq` on a dictionary value, which never goes
//!   through an application, and the `[T]` expression resolves by hand.
//! * **Instances** are `let`-shaped declarations whose type ends in a
//!   class application; the checked value is closed (the same
//!   no-enclosing-binders rule as stored constructor types) and pushed to
//!   a thread-local table, newest first — later declarations win, the
//!   usual local-covers-global approximation.
//! * **Resolution** (`resolve`) is a depth-limited backtracking search:
//!   for each table entry, fresh metavariables are created for the
//!   instance type's whole telescope, the codomain is unified with the
//!   target (this is exactly the ported pattern unification at work), the
//!   ordinary domains must come out solved and become the instance's
//!   arguments, and each *class* domain is resolved recursively.  A failed
//!   entry rolls the metacontext back — [`crate::meta`] keeps snapshot
//!   and restore for exactly this — so half-solved attempts are never
//!   observed.

use crate::context::Ctx;
use crate::elab;
use crate::eval::{eval, frc};
use crate::meta::{self, MetaEntry};
use crate::syntax::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Search depth bound: an instance like `instance : Eq A -> Eq (Foo A)`
/// applied to a deeply nested target recurses once per layer; cycles
/// (`instance : Eq A -> Eq A`) run into this and fail.
const MAX_DEPTH: u32 = 64;

/// A class: the single-constructor declaration behind the sugar, plus the
/// number of type parameters (the constructor's result is `C p1 .. pk`).
#[derive(Clone, Debug)]
pub struct ClassDef {
    pub def: Rc<IndDef>,
}

/// One instance-table entry.
#[derive(Clone, Debug)]
pub struct InstanceDef {
    pub name: Name,
    /// The closed instance value.
    pub term: Tm,
    /// The closed annotated type: `Pi params-and-constraints. C args`.
    pub ty: Tm,
}

struct TcCtx {
    classes: Vec<(Name, ClassDef)>,
    instances: Vec<InstanceDef>,
}

thread_local! {
    static TC: RefCell<TcCtx> = RefCell::new(TcCtx {
        classes: Vec::new(),
        instances: Vec::new(),
    });
}

/// Drop every class and instance: the tables are per program.
pub fn reset() {
    TC.with(|t| {
        t.borrow_mut().classes.clear();
        t.borrow_mut().instances.clear();
    });
}

/// Register a class (called after the struct sugar has elaborated).
pub fn add_class(name: Name, def: Rc<IndDef>) {
    TC.with(|t| t.borrow_mut().classes.push((name, ClassDef { def })));
}

/// Register an instance (newest last; resolution scans in reverse).
pub fn add_instance(def: InstanceDef) {
    TC.with(|t| t.borrow_mut().instances.push(def));
}

/// Whether the datatype is a declared class.
pub fn is_class(def: &Rc<IndDef>) -> bool {
    TC.with(|t| {
        t.borrow()
            .classes
            .iter()
            .any(|(_, c)| Rc::ptr_eq(&c.def, def))
    })
}

/// The class name a datatype was declared under (for error messages).
pub fn class_name(def: &Rc<IndDef>) -> Option<Name> {
    TC.with(|t| {
        t.borrow()
            .classes
            .iter()
            .find(|(_, c)| Rc::ptr_eq(&c.def, def))
            .map(|(n, _)| n.clone())
    })
}

/// Whether a value is an application of a declared class (and the class
/// itself, for error messages).
pub fn as_class(v: &Val) -> Option<(Rc<IndDef>, Spine)> {
    match &frc(v) {
        Val::Ne(n) => match &**n {
            Ne::Data(d, sp) if is_class(d) => Some((d.clone(), sp.clone())),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve an instance of the class type `target`, in `ctx`.
///
/// Returns the elaborated dictionary term, or `None` when no table entry
/// matches (with the metacontext restored, so nothing is observed).
pub fn resolve(ctx: &Ctx, target: &Val) -> Option<Tm> {
    resolve_go(ctx, target, 0)
}

fn resolve_go(ctx: &Ctx, target: &Val, depth: u32) -> Option<Tm> {
    if depth > MAX_DEPTH {
        return None;
    }
    let entries: Vec<InstanceDef> =
        TC.with(|t| t.borrow().instances.iter().rev().cloned().collect());
    for e in entries {
        let snap = meta::snapshot();
        match try_entry(ctx, &e, target, depth) {
            Some(t) => return Some(t),
            None => meta::restore(snap),
        }
    }
    None
}

/// Try one instance entry against the target: fresh metas for the
/// telescope, unify the codomain, then arguments and recursive
/// constraints.
fn try_entry(ctx: &Ctx, e: &InstanceDef, target: &Val, depth: u32) -> Option<Tm> {
    // Build the telescope with fresh metas: each domain (ordinary or
    // constraint) becomes a metavariable; the chain value is the codomain.
    let mut dom_tms: Vec<Tm> = Vec::new();
    let mut dom_cls: Vec<bool> = Vec::new();
    let mut ty = eval(&Env::empty(), &e.ty);
    loop {
        let (dom, cod) = match &frc(&ty) {
            Val::Pi(_, a, b) => ((**a).clone(), b.clone()),
            _ => break,
        };
        let is_cls = as_class(&dom).is_some();
        let m = elab::fresh_meta(ctx, &dom);
        let mv = eval(&ctx.env, &m);
        dom_tms.push(m);
        dom_cls.push(is_cls);
        ty = frc(&cod.apply(mv));
    }

    // The codomain must *be* the target (metas on the instance side get
    // solved; the target side may contain the caller's metas, which also
    // get solved -- a resolution that pins down inference is fine).
    if !crate::unify::unify(ctx.lvl, &ty, target) {
        return None;
    }

    // Every domain must now be determined: ordinary ones by the match,
    // class ones by the recursive resolution below.
    let mut args: Vec<Tm> = Vec::with_capacity(dom_tms.len());
    for (i, m) in dom_tms.iter().enumerate() {
        let solved = match meta::get(meta_id_of(m)) {
            MetaEntry::Solved { val, .. } => crate::eval::quote_val(ctx.lvl, &val),
            MetaEntry::Unsolved { .. } => {
                if dom_cls[i] {
                    // resolve the constraint's value now: the domain value
                    // with the already-known metas instantiated
                    Tm::Bot // placeholder, replaced below
                } else {
                    return None; // under-determined match
                }
            }
        };
        args.push(solved);
    }

    // Recompute the class-domain values after unification and resolve
    // them, in order, extending the dictionary term.
    let mut term = e.term.clone();
    let mut ty = eval(&Env::empty(), &e.ty);
    for (i, cls) in dom_cls.iter().enumerate() {
        let (dom, cod) = match &frc(&ty) {
            Val::Pi(_, a, b) => ((**a).clone(), b.clone()),
            _ => unreachable!("telescope was walked above"),
        };
        if *cls {
            //  has already been advanced past args[..i], so  is
            // the constraint instantiated with the chosen arguments
            let sg = resolve_go(ctx, &dom, depth + 1)?;
            args[i] = sg;
            // solve the constraint meta, so nothing unsolved is left behind
            let mid = meta_id_of(&dom_tms[i]);
            if let MetaEntry::Unsolved { ty, .. } = meta::get(mid) {
                let val = eval(&ctx.env, &args[i]);
                meta::solve(mid, val, ty);
            }
        } else if matches!(meta::get(meta_id_of(&dom_tms[i])), MetaEntry::Unsolved { .. }) {
            return None;
        }
        let av = eval(&ctx.env, &args[i]);
        term = Tm::App(Box::new(term), Box::new(args[i].clone()));
        ty = frc(&cod.apply(av));
    }
    Some(term)
}

/// The meta id of a `fresh_meta`-produced term (`?m applied to ctx vars`).
fn meta_id_of(m: &Tm) -> MetaId {
    match m {
        Tm::Meta(id) => *id,
        Tm::App(f, _) => meta_id_of(f),
        _ => unreachable!("fresh_meta terms are meta-headed spines"),
    }
}

/// The instance-resolution term for the class-application domain, as
/// `insert_instances` builds it: resolve or fail with the documented
/// message.
pub fn resolve_or_report(ctx: &Ctx, target: &Val) -> crate::context::M<Tm> {
    match resolve(ctx, target) {
        Some(t) => Ok(t),
        None => {
            let shown = crate::pretty::show_val(ctx, target);
            let cls = as_class(target)
                .and_then(|(d, _)| class_name(&d))
                .unwrap_or_else(|| "a class".to_string());
            crate::elab::report_tc(ctx, format!("no instance of {shown} (class {cls})"))
        }
    }
}

/// Fill the leading class-application domains of a function type with
/// resolved instances, returning the applied term and the remaining type.
pub fn insert_instances(ctx: &Ctx, t: Tm, ty: Val) -> crate::context::M<(Tm, Val)> {
    let mut t = t;
    let mut ty = ty;
    loop {
        let (dom, cod) = match &frc(&ty) {
            Val::Pi(_, a, b) => ((**a).clone(), b.clone()),
            _ => break,
        };
        if as_class(&dom).is_none() {
            break;
        }
        let dict = resolve_or_report(ctx, &dom)?;
        let dv = eval(&ctx.env, &dict);
        t = Tm::App(Box::new(t), Box::new(dict));
        ty = frc(&cod.apply(dv));
    }
    Ok((t, ty))
}

/// Whether any instance has been declared (used to skip work in
/// meta-free programs).
pub fn has_instances() -> bool {
    TC.with(|t| !t.borrow().instances.is_empty())
}
