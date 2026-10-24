//! Hand-assembled inductive definitions shared by the unit tests, in the
//! stored representation used by [`crate::syntax::IndDef`]:
//!
//! * `nat_def` -- `data Nat : U where zero : Nat; suc : Nat -> Nat;`
//! * `list_def` -- `data List (A : U) : U where nil : List A;
//!   cons : A -> List A -> List A;`
//!
//! Constructor types keep the datatype itself as their single free variable
//! (index = local depth), exactly as [`crate::elab::elab_data`] stores them.

use crate::syntax::{CtorInfo, IndDef, Tm};
use std::rc::Rc;

pub fn nat_def() -> Rc<IndDef> {
    Rc::new(IndDef {
        name: "Nat".into(),
        level: 0,
        params: vec![],
        ctors: vec![
            CtorInfo {
                name: "zero".into(),
                ty: Tm::Var(crate::Ix(0)),
                num_args: 0,
                recursive: vec![],
            },
            CtorInfo {
                name: "suc".into(),
                ty: Tm::Pi(
                    "n".into(),
                    Box::new(Tm::Var(crate::Ix(0))),
                    Box::new(Tm::Var(crate::Ix(1))),
                ),
                num_args: 1,
                recursive: vec![true],
            },
        ],
    })
}

pub fn list_def() -> Rc<IndDef> {
    // nil  : (A : U) -> D A          (stored: Pi A. #1 #0)
    // cons : (A : U) -> A -> D A -> D A
    let nil = Tm::Pi(
        "A".into(),
        Box::new(Tm::U(0)),
        Box::new(Tm::App(
            Box::new(Tm::Var(crate::Ix(1))),
            Box::new(Tm::Var(crate::Ix(0))),
        )),
    );
    let cons = Tm::Pi(
        "A".into(),
        Box::new(Tm::U(0)),
        Box::new(Tm::Pi(
            "a".into(),
            Box::new(Tm::Var(crate::Ix(0))),
            Box::new(Tm::Pi(
                "as".into(),
                Box::new(Tm::App(
                    Box::new(Tm::Var(crate::Ix(2))),
                    Box::new(Tm::Var(crate::Ix(1))),
                )),
                Box::new(Tm::App(
                    Box::new(Tm::Var(crate::Ix(3))),
                    Box::new(Tm::Var(crate::Ix(2))),
                )),
            )),
        )),
    );
    Rc::new(IndDef {
        name: "List".into(),
        level: 0,
        params: vec![("A".into(), Tm::U(0))],
        ctors: vec![
            CtorInfo {
                name: "nil".into(),
                ty: nil,
                num_args: 0,
                recursive: vec![],
            },
            CtorInfo {
                name: "cons".into(),
                ty: cons,
                num_args: 2,
                recursive: vec![false, true],
            },
        ],
    })
}
