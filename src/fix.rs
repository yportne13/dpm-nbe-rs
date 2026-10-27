//! The fixcontext: checked fixpoints (`let rec` definitions).
//!
//! The port of a `let rec f : T := body` stores the definition in a
//! [`thread_local`] table, exactly the way [`crate::meta`] stores holes: the
//! core term only carries a [`FixId`], so two occurrences of the same
//! fixpoint compare by identity (as datatypes do by pointer) and the
//! definition is closed — no dangling context variables.
//!
//! Closeness is achieved the same way stored constructor types are closed
//! (`elab::subst_outer`'s convention): a `let rec` may only appear where the
//! context contains no *bound* variables (top level; defines are fine and
//! are inlined by evaluation), the body is checked with `f` bound as a
//! fresh variable, and the closed body is the quotation of its value with
//! that variable's occurrences replaced by `FixRef(id)` -- so the
//! self-reference is the table lookup itself, and unfolding needs no
//! environment.
//!
//! Reduction: `?fix t⃗` with `t₀` constructor-headed unfolds to the closed
//! body applied to `t⃗` (`eval::fix_step`).  Well-foundedness of unfolding
//! is the termination checker's theorem: every self-call is on a strict
//! subterm (depth ≥ 1) of the designated recursive argument, tracked
//! through eliminator methods (NOTES.md deviation 19).

use crate::syntax::{Name, Tm};
use std::cell::RefCell;

pub use crate::syntax::FixId;

/// One checked fixpoint definition.
#[derive(Clone, Debug, PartialEq)]
pub struct FixDef {
    pub name: Name,
    /// The closed annotated type, an iterated-Pi telescope.
    pub ty: Tm,
    /// The closed body: a λ-telescope of the same arity, with self-calls
    /// as `Tm::FixRef(id)`.
    pub body: Tm,
    /// Which telescope argument the termination check ran on.
    pub rec_arg: usize,
}

thread_local! {
    static FIXCTX: RefCell<Vec<FixDef>> = const { RefCell::new(Vec::new()) };
}

/// Drop every fixpoint: the fixcontext is per program.
pub fn reset() {
    FIXCTX.with(|m| m.borrow_mut().clear());
}

/// Reserve a slot; filled immediately after with [`fill`] (the body is only
/// known once elaboration and the termination check have run).
pub fn reserve() -> FixId {
    FIXCTX.with(|m| {
        let mut m = m.borrow_mut();
        let id = m.len();
        m.push(FixDef {
            name: String::new(),
            ty: Tm::Bot,
            body: Tm::Bot,
            rec_arg: 0,
        });
        id
    })
}

/// Fill a reserved slot.
pub fn fill(id: FixId, def: FixDef) {
    FIXCTX.with(|m| m.borrow_mut()[id] = def);
}

/// Look a fixpoint up.
pub fn get(id: FixId) -> FixDef {
    FIXCTX.with(|m| {
        m.borrow()
            .get(id)
            .cloned()
            .expect("fix::get: unknown fixpoint")
    })
}

/// The name a fixpoint prints under.
pub fn name_of(id: FixId) -> Name {
    get(id).name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_fill_reset_roundtrip() {
        reset();
        let id = reserve();
        assert_eq!(
            get(id),
            FixDef { name: String::new(), ty: Tm::Bot, body: Tm::Bot, rec_arg: 0 }
        );
        let def = FixDef {
            name: "loop".into(),
            ty: Tm::U(0),
            body: Tm::Bot,
            rec_arg: 0,
        };
        fill(id, def.clone());
        assert_eq!(get(id), def);
        reset();
        assert_eq!(reserve(), 0, "ids restart after a reset");
        reset();
    }
}
