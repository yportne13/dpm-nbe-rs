//! The metacontext: metavariables (holes), their types and their solutions.
//!
//! Port of `Metacontext.hs` from elaboration-zoo's `05-pruning`, adapted in
//! two ways.  The zoo keeps the metacontext in a global `IORef` (read by
//! `force` and `eval` wherever a `Meta` node is met); the port keeps it in a
//! [`thread_local`] `RefCell`, which is the same shape of global without
//! threading an extra parameter through every evaluator signature --
//! `typecheck_named` resets it per program, so runs are independent.
//! Types are stored as *terms* (closed iterated `Pi` chains) rather than
//! values: [`crate::unify::prune_ty`] then walks the chain directly instead
//! of quoting under a partial renaming.

use crate::syntax::{Lvl, MetaId, Name, Pos, Tm, Val};
use std::cell::RefCell;
use std::rc::Rc;

/// One entry of a context chain kept for closing meta types, mirroring the
/// zoo's `Locals`: a `Bind` becomes a `Pi` in the closed type, a `Define`
/// becomes a `Let`, which is what keeps de Bruijn indices aligned (the type
/// is quoted at the *whole* context's size, defines included).
#[derive(Clone, Debug)]
pub enum LocalEntry {
    Bind(Rc<Tm>),
    Define { ty: Rc<Tm>, val: Rc<Tm> },
    /// Set by `subst_ctx`: the picture is stale after a refinement, and the
    /// next fresh meta rebuilds its chain by re-quoting.
    Stale,
}

/// A meta's type, closed on demand.  Fresh metas keep their type as it sits
/// in the elaboration context -- a value plus the shared context chain --
/// and only the operations that walk the Pi chain (pruning, `lams`, the
/// unsolved report) pay to close it.  Closing eagerly instead makes every
/// binder over a hole copy the whole context again, which is cubic in
/// deeply nested programs (the zoo's laziness hides the same cost).
#[derive(Clone, Debug)]
pub enum MetaTy {
    Closed(Tm),
    Open {
        /// The type as a value in the full context (of size `lvl`).
        ty: Val,
        /// The context chain, newest first, as `Ctx` orders it.
        chain: Vec<(Name, LocalEntry)>,
        lvl: Lvl,
    },
}

impl MetaTy {
    /// The closed type: `Pi` for every binder, `Let` for every define, in
    /// context order.  The value is quoted at the whole context's size, so
    /// no index moves.
    pub fn close(&self) -> Tm {
        match self {
            MetaTy::Closed(t) => t.clone(),
            MetaTy::Open { ty, chain, lvl } => {
                // `chain` is newest-first, so wrapping in order puts the
                // oldest entry at the head.
                let mut closed = crate::eval::quote_val(*lvl, ty);
                for (x, e) in chain {
                    closed = match e {
                        LocalEntry::Bind(a) => {
                            Tm::Pi(x.clone(), Box::new((**a).clone()), Box::new(closed))
                        }
                        LocalEntry::Define { ty, val } => Tm::Let(
                            x.clone(),
                            Box::new((**ty).clone()),
                            Box::new((**val).clone()),
                            Box::new(closed),
                        ),
                        LocalEntry::Stale => {
                            unreachable!("fresh_meta rebuilds stale chains")
                        }
                    };
                }
                closed
            }
        }
    }
}

/// One metavariable.
#[derive(Clone, Debug)]
pub enum MetaEntry {
    /// The solution, evaluated in the *empty* environment (metas are closed),
    /// plus the type the meta was created with.
    Solved { val: Val, ty: MetaTy },
    /// The type, and the source position the hole came from (for the
    /// end-of-program "unsolved" report).
    Unsolved { ty: MetaTy, pos: Pos },
}

thread_local! {
    static MCXT: RefCell<Vec<MetaEntry>> = const { RefCell::new(Vec::new()) };
}

/// Drop every meta: the metacontext is per program.
pub fn reset() {
    MCXT.with(|m| m.borrow_mut().clear());
}

/// Allocate a metavariable of the given open type (see [`MetaTy::Open`]).
pub fn fresh(ty: MetaTy, pos: Pos) -> MetaId {
    MCXT.with(|m| {
        let mut m = m.borrow_mut();
        let id = m.len();
        m.push(MetaEntry::Unsolved { ty, pos });
        id
    })
}

/// Look a metavariable up.
pub fn get(m: MetaId) -> MetaEntry {
    MCXT.with(|mcxt| {
        mcxt
            .borrow()
            .get(m)
            .cloned()
            .expect("meta::get: unknown metavariable")
    })
}

/// Record a solution.
pub fn solve(m: MetaId, val: Val, ty: MetaTy) {
    MCXT.with(|mcxt| {
        mcxt.borrow_mut()[m] = MetaEntry::Solved { val, ty };
    });
}

/// Whether the metavariable has a solution.
pub fn is_solved(m: MetaId) -> bool {
    matches!(get(m), MetaEntry::Solved { .. })
}

/// Whether any metavariable exists -- programs without holes never need the
/// unifier, so solving-enabled conversion can skip its fallback.
pub fn is_empty() -> bool {
    MCXT.with(|m| m.borrow().is_empty())
}

/// A snapshot of the metacontext, for speculative solving (instance
/// resolution tries entries and must roll failed tries back).  Entries are
/// cheap to clone (values are `Rc`), and `restore` also restores id
/// allocation, since `fresh` derives ids from the vector's length.
pub fn snapshot() -> Vec<MetaEntry> {
    MCXT.with(|m| m.borrow().clone())
}

/// Roll the metacontext back to a snapshot.
pub fn restore(snap: Vec<MetaEntry>) {
    MCXT.with(|m| {
        *m.borrow_mut() = snap;
    });
}

/// The unsolved metavariables, in creation order.
pub fn unsolved() -> Vec<MetaId> {
    MCXT.with(|mcxt| {
        mcxt
            .borrow()
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, MetaEntry::Unsolved { .. }))
            .map(|(i, _)| i)
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::Ix;

    #[test]
    fn fresh_get_solve_reset_roundtrip() {
        reset();
        let m = fresh(MetaTy::Closed(Tm::U(0)), Pos::initial("<t>"));
        assert_eq!(unsolved(), vec![m]);
        solve(m, Val::U(0), MetaTy::Closed(Tm::U(0)));
        assert!(is_solved(m));
        assert!(unsolved().is_empty());
        assert!(matches!(get(m), MetaEntry::Solved { .. }));
        reset();
        assert!(unsolved().is_empty());
        // ids restart after a reset
        assert_eq!(fresh(MetaTy::Closed(Tm::U(0)), Pos::initial("<t>")), 0);
        reset();
    }

    #[test]
    fn solutions_are_independent_of_context_order() {
        // two metas alive at once keep distinct entries
        reset();
        let a = fresh(MetaTy::Closed(Tm::U(0)), Pos::initial("<t>"));
        let b = fresh(MetaTy::Closed(Tm::U(1)), Pos::initial("<t>"));
        assert_ne!(a, b);
        solve(a, Val::U(0), MetaTy::Closed(Tm::U(0)));
        assert!(matches!(get(b), MetaEntry::Unsolved { .. }));
        reset();
    }

    #[test]
    fn open_types_close_over_their_chain() {
        // (f : U)(x : f) ⊢ U1  closes to  (f : U)(x : f) → U1
        let chain = vec![
            ("x".to_string(), LocalEntry::Bind(Rc::new(Tm::Var(Ix(0))))),
            ("f".to_string(), LocalEntry::Bind(Rc::new(Tm::U(0)))),
        ];
        let mty = MetaTy::Open {
            ty: Val::U(1),
            chain,
            lvl: Lvl(2),
        };
        assert_eq!(
            mty.close(),
            Tm::Pi(
                "f".into(),
                Box::new(Tm::U(0)),
                Box::new(Tm::Pi(
                    "x".into(),
                    Box::new(Tm::Var(Ix(0))),
                    Box::new(Tm::U(1))
                ))
            )
        );
        // a define in the chain becomes a Let, keeping indices aligned:
        // the context is [f : U, d := f, x : f], so `x`'s type is quoted
        // as Var(1) (f, under the d slot) and `d`'s value as Var(0)
        let chain = vec![
            ("x".to_string(), LocalEntry::Bind(Rc::new(Tm::Var(Ix(1))))),
            ("d".to_string(), LocalEntry::Define {
                ty: Rc::new(Tm::U(0)),
                val: Rc::new(Tm::Var(Ix(0))),
            }),
            ("f".to_string(), LocalEntry::Bind(Rc::new(Tm::U(0)))),
        ];
        let mty = MetaTy::Open {
            ty: Val::U(1),
            chain,
            lvl: Lvl(3),
        };
        assert_eq!(
            mty.close(),
            Tm::Pi(
                "f".into(),
                Box::new(Tm::U(0)),
                Box::new(Tm::Let(
                    "d".into(),
                    Box::new(Tm::U(0)),
                    Box::new(Tm::Var(Ix(0))),
                    Box::new(Tm::Pi(
                        "x".into(),
                        Box::new(Tm::Var(Ix(1))),
                        Box::new(Tm::U(1))
                    ))
                ))
            )
        );
    }
}
