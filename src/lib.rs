//! # dpm-nbe — dependent pattern matching under normalization by evaluation
//!
//! A Rust port of the Haskell project of the same name.  The pipeline is the
//! classic elaboration zoo pipeline:
//!
//! ```text
//! parse (Raw) --elaborate--> core term + semantic type --evaluate--> value --quote--> normal form
//! ```
//!
//! The interesting part is [`elab::check_pm`]: checking a lambda whose
//! patterns may be `refl` or absurd `(!)` requires *unifying* the indices of
//! the identity type, which produces an explicit substitution.  Because
//! explicit substitutions are part of values ([`syntax::Val::Sub`]) and values
//! are forced on demand ([`eval::frc`]), a substitution that is produced later
//! can still invalidate a value that was already computed earlier — the
//! difficulty the original project is about.

pub mod context;
pub mod elab;
pub mod eval;
pub mod parse;
pub mod pretty;
pub mod subst;
pub mod syntax;

#[cfg(test)]
pub mod testdata;

pub use crate::context::{Ctx, Error};
pub use crate::syntax::{Env, Ix, Lvl, Name, Pn, Pos, Raw, Tm, Ty, Val};

impl From<parse::ParseError> for Error {
    fn from(e: parse::ParseError) -> Error {
        Error {
            msg: e.msg,
            pos: e.pos,
        }
    }
}

/// What the command line should print.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Normal form and type of the program.
    Nf,
    /// Type of the program.
    Type,
}

/// Stack size used by the CLI for the elaborator thread.
///
/// Parsing, elaboration, evaluation and quotation all recurse over the
/// structure of the program (and `parse::MAX_DEPTH` bounds how deep that can
/// get), so library users processing untrusted input should run the pipeline
/// on a thread with a comparable stack, as `main.rs` does.
pub const STACK_SIZE: usize = 64 * 1024 * 1024;

/// Elaborate a program from `src`, reporting positions as `file`.
pub fn typecheck_named(src: &str, file: &str) -> Result<(Tm, Val), Error> {
    let raw = parse::parse_named(src, file).map_err(Error::from)?;
    let ctx = Ctx::empty(Pos::initial(file));
    elab::infer(&ctx, &raw)
}

/// Elaborate a program, reporting positions as `<stdin>`.
pub fn typecheck(src: &str) -> Result<(Tm, Val), Error> {
    typecheck_named(src, "<stdin>")
}

/// Normalize a program (used by tests): the normal form of its value.
pub fn normalize(src: &str) -> Result<Tm, Error> {
    let (t, _) = typecheck(src)?;
    Ok(eval::nf(&Env::empty(), &t))
}

/// Normalize a program and also return its type, read back from a semantic
/// value (so the returned type is already a normal form).
pub fn normalize_and_type(src: &str) -> Result<(Tm, Tm), Error> {
    let (t, a) = typecheck(src)?;
    Ok((eval::nf(&Env::empty(), &t), eval::quote_val(Lvl::ZERO, &a)))
}

/// Run the elaborator in the given output mode, producing the text the CLI
/// prints on success.
pub fn run_named(src: &str, file: &str, mode: Mode) -> Result<String, Error> {
    match mode {
        Mode::Nf => {
            let (t, a) = typecheck_named(src, file)?;
            let nf = eval::nf(&Env::empty(), &t);
            let ty = eval::quote_val(Lvl::ZERO, &a);
            Ok(format!(
                "{}\n  :\n{}",
                pretty::show_tm(&nf),
                pretty::show_tm(&ty)
            ))
        }
        Mode::Type => {
            let (_, a) = typecheck_named(src, file)?;
            Ok(pretty::show_tm(&eval::quote_val(Lvl::ZERO, &a)))
        }
    }
}

/// Run the elaborator with the default source name.
pub fn run(src: &str, mode: Mode) -> Result<String, Error> {
    run_named(src, "<stdin>", mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- helpers -----------------------------------------------------------

    fn normal_form(src: &str) -> String {
        pretty::show_tm(&normalize(src).unwrap_or_else(|e| panic!("{src}\n=> {}", e.msg)))
    }

    fn type_of(src: &str) -> String {
        let (_, a) = normalize_and_type(src).unwrap_or_else(|e| panic!("{src}\n=> {}", e.msg));
        pretty::show_tm(&a)
    }

    fn error(src: &str) -> String {
        match typecheck(src) {
            Ok(_) => panic!("{src}\n=> expected an error, but it typechecked"),
            Err(e) => e.msg,
        }
    }

    fn error_pos(src: &str) -> (usize, usize) {
        match typecheck(src) {
            Ok(_) => panic!("{src}\n=> expected an error"),
            Err(e) => (e.pos.line, e.pos.col),
        }
    }

    /// The declaration every `Nat`-using program starts with.  `Nat` is not
    /// built in: programs bring their own arithmetic.
    const NAT: &str = "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;";

    /// `NAT` plus the classic addition, defined with the generated eliminator.
    const NAT_PLUS: &str = "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat; \
let plus : Nat -> Nat -> Nat = \
λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;";

    /// `n` in the source syntax.
    fn numeral(n: usize) -> String {
        if n == 0 {
            "zero".to_string()
        } else {
            format!("suc ({})", numeral(n - 1))
        }
    }

    /// `n` as the pretty printer renders it.
    fn printed_nat(n: usize) -> String {
        match n {
            0 => "zero".to_string(),
            1 => "suc zero".to_string(),
            _ => format!("suc ({})", printed_nat(n - 1)),
        }
    }

    /// `transN` as generated by `genTransN` in the original `Main.hs`.
    fn gen_trans(n: usize) -> String {
        let binders: String = (1..=n).map(|i| format!("(a{i} : A) ")).collect();
        let args: String = (1..n)
            .map(|i| format!("-> Id A a{i} a{} ", i + 1))
            .collect();
        let ret = format!("-> Id A a1 a{n}");
        let vars: String = (1..=n).map(|i| format!("a{i} ")).collect();
        let refls: String = (1..n).map(|_| "refl ".to_string()).collect();
        format!("let trans{n} : (A : U) {binders}{args}{ret} =  λ A {vars}{refls}. refl; U")
    }

    // -- the README examples -----------------------------------------------

    #[test]
    fn readme_sym_checks() {
        let src = "\
let sym : (A : U) (a b : A) -> Id A a b -> Id A b a =
  λ A a b refl . refl; U";
        assert_eq!(normal_form(src), "U");
        assert_eq!(type_of(src), "U");
    }

    #[test]
    fn readme_trans_checks() {
        let src = "\
let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  λ A a b c refl refl . refl; U";
        assert_eq!(normal_form(src), "U");
    }

    #[test]
    fn readme_cong_checks() {
        let src = "\
let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) =
  λ A B f a b refl . refl; U";
        assert_eq!(normal_form(src), "U");
    }

    #[test]
    fn readme_h_is_the_dependent_pattern_matching_example() {
        // Forcing makes `plus m zero [zero/m]` reduce again after the second
        // `refl` refines `m`; this is the case the whole project is about.
        let src = format!(
            "{NAT_PLUS} \
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = \
λ m n refl refl . refl; U"
        );
        assert_eq!(normal_form(&src), "U");
    }

    #[test]
    fn readme_absurd_pattern_checks() {
        let src = format!(
            "{NAT} \
let C : (A : U) -> Id Nat (suc zero) zero -> A = \
λ A (!); U"
        );
        assert_eq!(normal_form(&src), "U");
    }

    #[test]
    fn the_full_readme_program_checks() {
        let src = format!(
            "{NAT_PLUS} \
let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = \
λ A a b c refl refl . refl; \
let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) = \
λ A B f a b refl . refl; \
let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = \
λ m n refl refl . refl; \
let C : (A : U) -> Id Nat (suc zero) zero -> A = \
λ A (!); \
U"
        );
        assert_eq!(normal_form(&src), "U");
    }

    // -- inductive declarations --------------------------------------------

    #[test]
    fn declarations_introduce_the_type_and_its_constructors() {
        assert_eq!(type_of(&format!("{NAT} Nat")), "U");
        assert_eq!(type_of(&format!("{NAT} zero")), "Nat");
        assert_eq!(type_of(&format!("{NAT} suc")), "(n : Nat) → Nat");
        assert_eq!(type_of(&format!("{NAT} suc zero")), "Nat");
        assert_eq!(
            type_of(&format!("{NAT} let two : Nat = suc (suc zero); two")),
            "Nat"
        );
    }

    #[test]
    fn declarations_are_scoped_like_lets() {
        // Shadowing: the body may rebind the constructor names.
        assert_eq!(
            normal_form(&format!("{NAT} let zero : Nat = suc zero; zero")),
            "suc zero"
        );
        // and the declaration only scopes over its own body
        assert_eq!(error("zero"), "unbound variable: zero");
    }

    #[test]
    fn identical_declarations_are_distinct_types() {
        // Two structurally identical declarations are still different types
        // (nominal typing): the first one's `zero` does not inhabit the
        // second one.
        let src = format!(
            "{NAT} \
data Nat2 : U where z2 : Nat2; s2 : (n : Nat2) -> Nat2; \
let bad : Nat2 = zero; U"
        );
        let msg = error(&src);
        assert!(msg.contains("type mismatch"), "{msg}");
    }

    #[test]
    fn the_eliminator_has_the_recursor_type() {
        assert_eq!(
            type_of(&format!("{NAT} elim Nat")),
            "(P : Nat → U) → P zero → ((n : Nat)(n' : P n) → P (suc n)) → (x : Nat) → P x"
        );
        // and it is eliminable: `plus zero n = n`
        let src = format!(
            "{NAT} \
let idn : Nat -> Nat = λ n . elim Nat (λ _ . Nat) n (λ _ r . suc r) zero; \
idn (suc (suc zero))"
        );
        assert_eq!(normal_form(&src), "suc (suc zero)");
    }

    #[test]
    fn parameterised_declarations_generate_parameterised_eliminators() {
        let list = "data List (A : U) : U where nil : List A; cons : (a : A) -> (as : List A) -> List A;";
        let length_def = "\
let length : (A : U) -> List A -> Nat = \
λ A xs . elim List A (λ _ . Nat) zero (λ a as r . suc r) xs;";
        // the eliminator's type abstracts the parameter first
        assert_eq!(
            type_of(&format!("{NAT} {list} elim List")),
            "(A : U)(P : List A → U) → P (nil A) → ((a : A)(as : List A)(as' : P as) \
→ P (cons A a as)) → (x : List A) → P x"
        );
        // ... and a whole program, declaration, definition and use, reduces
        let use_it = format!(
            "{NAT} {list} {length_def} \
length Nat (cons Nat zero (cons Nat zero (nil Nat)))"
        );
        assert_eq!(normal_form(&use_it), "suc (suc zero)");
    }

    #[test]
    fn the_empty_type_has_a_usable_eliminator() {
        let src = "\
data Empty : U where U";
        assert_eq!(type_of(src), "U");
        assert_eq!(
            type_of("data Empty : U where elim Empty"),
            "(P : Empty → U)(x : Empty) → P x"
        );
        // From an `Empty` anything follows, without a body.
        assert_eq!(
            normal_form(
                "data Empty : U where \
let ex : (A : U) -> Empty -> A = λ A x . elim Empty (λ _ . A) x; U"
            ),
            "U"
        );
    }

    #[test]
    fn constructor_result_must_be_the_datatype() {
        let msg = error("data Bad : U where mk : U; U");
        assert!(msg.contains("constructor mk must return Bad"), "{msg}");
        // a parameterised datatype must be applied to its parameters: the
        // bare `P` is not even a type (it is `(A : U) -> U`), so the
        // constructor type does not check against `U`.
        let msg = error("data P (A : U) : U where mk : P; U");
        assert!(msg.contains("type mismatch"), "{msg}");
        assert!(msg.contains("(A' : U) → U"), "{msg}");
        // applied, it is fine
        assert_eq!(normal_form("data P (A : U) : U where mk : P A; U"), "U");
    }

    #[test]
    fn strict_positivity_is_enforced() {
        let msg = error("data Bad : U where mk : (Bad -> U) -> Bad; U");
        assert!(msg.contains("not strictly positive"), "{msg}");
        let msg = error("data Bad : U where mk : Id U Bad Bad -> Bad; U");
        assert!(msg.contains("not strictly positive"), "{msg}");
        // occurrences in a *function's domain* are the negative ones; a
        // function *into* the datatype is fine (no induction hypothesis)
        assert_eq!(
            normal_form(&format!(
                "{NAT} data Tree : U where \
leaf : Tree; node : (Nat -> Tree) -> Tree; U"
            )),
            "U"
        );
    }

    #[test]
    fn indexed_families_are_rejected() {
        let msg = error(&format!(
            "{NAT} data Bad : Nat -> U where mk : Bad zero; U"
        ));
        assert!(msg.contains("must be declared in U"), "{msg}");
    }

    #[test]
    fn constructors_must_not_capture_enclosing_binders() {
        let msg = error(
            "let f : (A : U) -> U = λ A . data D : U where mk : A -> D; D; U",
        );
        assert!(
            msg.contains("must not depend on enclosing bindings"),
            "{msg}"
        );
        // ... and so are enclosing *definitions*: inline the type instead
        assert_eq!(
            normal_form("let A : U = U; data D : U where mk : U -> D; D"),
            "D"
        );
    }

    #[test]
    fn elim_rejects_non_data_heads() {
        assert!(error("elim U").contains("elim expects a data type"));
        assert!(
            error(&format!(
                "{NAT} data List (A : U) : U where nil : List A; \
elim (List Nat)"
            ))
            .contains("elim expects the data type itself")
        );
    }

    // -- normalization -----------------------------------------------------

    #[test]
    fn normalization_of_closed_terms() {
        assert_eq!(normal_form("U"), "U");
        assert_eq!(normal_form(&format!("{NAT} Nat")), "Nat");
        assert_eq!(normal_form(&format!("{NAT} zero")), "zero");
        assert_eq!(normal_form(&format!("{NAT} suc zero")), "suc zero");
        let np = format!("{NAT} plus zero zero");
        assert_eq!(normal_form(&format!("{NAT_PLUS} plus zero zero")), "zero");
        let _ = np;
        assert_eq!(
            normal_form(&format!("{NAT_PLUS} plus (suc zero) (suc zero)")),
            "suc (suc zero)"
        );
        assert_eq!(
            normal_form(&format!("{NAT_PLUS} plus (suc (suc zero)) (suc zero)")),
            "suc (suc (suc zero))"
        );
        assert_eq!(
            normal_form(&format!("{NAT} Id Nat zero zero")),
            "Id Nat zero zero"
        );
    }

    #[test]
    fn plus_is_addition_on_numerals() {
        for a in 0..5 {
            for b in 0..5 {
                let src = format!("{NAT_PLUS} plus ({}) ({})", numeral(a), numeral(b));
                assert_eq!(
                    normal_form(&src),
                    printed_nat(a + b),
                    "wrong result for {a} + {b}"
                );
            }
        }
    }

    #[test]
    fn stuck_eliminators_are_comparable() {
        // `plus n m` is a stuck eliminator applied to variables; it must at
        // least be convertible with itself (the port's deviation 13, now for
        // generated eliminators instead of a built-in `plus`).
        let src = format!(
            "{NAT_PLUS} \
let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\ n m . refl; f"
        );
        assert_eq!(normal_form(&src), "λ n m . refl");
    }

    #[test]
    fn beta_reduction_substitutes_the_argument() {
        let src = format!(
            "{NAT_PLUS} let f : Nat -> Nat = \\ x . plus x (suc zero); f (suc zero)"
        );
        assert_eq!(normal_form(&src), "suc (suc zero)");
    }

    #[test]
    fn let_bindings_are_transparent() {
        assert_eq!(
            normal_form(&format!("{NAT_PLUS} let x : Nat = zero; plus x x")),
            "zero"
        );
        assert_eq!(
            normal_form(&format!(
                "{NAT_PLUS} let x : Nat = zero; let y : Nat = suc x; plus y y"
            )),
            "suc (suc zero)"
        );
    }

    #[test]
    fn lets_shadow_correctly() {
        let src = format!(
            "{NAT_PLUS} let x : Nat = zero; let x : Nat = suc x; plus x x"
        );
        assert_eq!(normal_form(&src), "suc (suc zero)");
    }

    #[test]
    fn lambda_values_print_with_their_binders() {
        assert_eq!(
            normal_form(&format!(
                "{NAT_PLUS} let f : Nat -> Nat = \\ n . plus n zero; f"
            )),
            "λ n . elim Nat (λ _ . Nat) zero (λ _ r . suc r) n"
        );
        assert_eq!(
            normal_form(&format!(
                "{NAT_PLUS} let f : Nat -> Nat = \\ n . plus n zero; f zero"
            )),
            "zero"
        );
    }

    #[test]
    fn partially_applied_lambdas_eta_expand() {
        let src = format!(
            "{NAT_PLUS} let f : Nat -> Nat -> Nat = \\ x y . plus x y; f zero"
        );
        assert_eq!(normal_form(&src), "λ y . y");
        assert_eq!(type_of(&src), "Nat → Nat");
    }

    #[test]
    fn comments_and_unicode_arrows_are_supported() {
        assert_eq!(
            normal_form(&format!("{NAT_PLUS} {{- hello -}} plus zero zero -- done")),
            "zero"
        );
        assert_eq!(type_of(&format!("{NAT} Nat → Nat → Nat")), "U");
    }

    // -- types -------------------------------------------------------------

    #[test]
    fn types_of_closed_terms() {
        assert_eq!(type_of("U"), "U");
        assert_eq!(type_of(&format!("{NAT} Nat")), "U");
        assert_eq!(type_of(&format!("{NAT} zero")), "Nat");
        assert_eq!(type_of(&format!("{NAT} suc zero")), "Nat");
        assert_eq!(type_of(&format!("{NAT} Id Nat zero zero")), "U");
        assert_eq!(type_of("(A : U) -> A -> A"), "U");
    }

    #[test]
    fn type_checking_is_directional() {
        // A lambda can be checked against a type but not inferred.
        assert!(typecheck("\\ x . x").is_err());
        let src = format!("{NAT} let f : Nat -> Nat = \\ x . x; f");
        assert_eq!(type_of(&src), "Nat → Nat");
    }

    #[test]
    fn refl_checks_definitional_equality() {
        // `plus zero zero` is convertible with `zero`
        assert!(typecheck(&format!(
            "{NAT_PLUS} let f : Id Nat zero (plus zero zero) = refl; U"
        ))
        .is_ok());
        // but not with `suc zero`
        assert!(
            error(&format!(
                "{NAT_PLUS} let f : Id Nat (suc zero) zero = refl; U"
            ))
            .contains("expecting suc zero equals to zero")
        );
    }

    // -- generated transitivity proofs -------------------------------------

    #[test]
    fn generated_transitivity_proofs_check() {
        for n in 1..=12 {
            let src = gen_trans(n);
            assert_eq!(normal_form(&src), "U", "trans{n} failed: {}", gen_trans(n));
        }
    }

    #[test]
    fn generated_transitivity_proofs_scale_polynomially() {
        // Composing substitutions naively made this exponential (2^n calls);
        // the layered representation keeps it quadratic.  A generous bound
        // catches a regression without being flaky.
        let start = std::time::Instant::now();
        for n in 1..=30 {
            assert!(typecheck(&gen_trans(n)).is_ok());
        }
        assert!(
            start.elapsed().as_secs() < 10,
            "transitivity proofs became too slow: {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn generated_transitivity_proofs_can_be_used() {
        // Bind the generated proof, then refer to it: the definition must be
        // applicable at its declared type.
        for n in 1..=4 {
            let ty: String = (1..=n).map(|i| format!("(a{i} : A) ")).collect();
            let args: String = (1..n)
                .map(|i| format!("-> Id A a{i} a{} ", i + 1))
                .collect();
            let ret = format!("-> Id A a1 a{n}");
            let vars: String = (1..=n).map(|i| format!("a{i} ")).collect();
            let refls: String = (1..n).map(|_| "refl ".to_string()).collect();
            let src = format!(
                "let go : (A : U) {ty}{args}{ret} = λ A {vars}{refls}. refl; \
                 let use : (A : U) {ty}{args}{ret} = λ A {vars}{refls}. go A {vars}{refls}; \
                 U"
            );
            assert_eq!(normal_form(&src), "U", "trans{n} could not be used");
            // The definition itself normalizes to a lambda of the right arity.
            let def = format!("let go : (A : U) {ty}{args}{ret} = λ A {vars}{refls}. refl; go");
            assert!(
                normal_form(&def).starts_with('λ'),
                "trans{n} did not normalize to a lambda"
            );
        }
    }

    // -- dependent pattern matching errors ---------------------------------

    #[test]
    fn unbound_variables_are_reported() {
        assert_eq!(error("x"), "unbound variable: x");
        assert_eq!(error_pos("x"), (1, 1));
    }

    #[test]
    fn type_mismatches_are_reported() {
        let msg = error(&format!("{NAT} let bad : Nat = U; U"));
        assert!(msg.contains("type mismatch"), "{msg}");
        assert!(msg.contains("expected type:\n\n  Nat"), "{msg}");
        assert!(msg.contains("inferred type:\n\n  U"), "{msg}");
    }

    #[test]
    fn cannot_infer_lambdas_or_refl() {
        assert_eq!(error("\\ x . x"), "Can't infer type for lambda expression");
        assert_eq!(error("refl"), "Can't infer type for refl");
    }

    #[test]
    fn function_expected_in_application_position() {
        let msg = error(&format!("{NAT} zero zero"));
        assert!(msg.contains("Expected a function type"), "{msg}");
    }

    #[test]
    fn refl_pattern_on_a_non_identity_type() {
        let msg = error(&format!("{NAT} let f : Nat -> Nat = \\ refl . zero; U"));
        assert!(
            msg.contains("Expected an identity type for pattern 'refl'"),
            "{msg}"
        );
    }

    #[test]
    fn absurd_pattern_on_a_provable_equation() {
        let msg = error(&format!(
            "{NAT} let C : (A : U) -> Id Nat zero zero -> A = \\ A (!); U"
        ));
        assert!(msg.contains("It should not be '(!)'"), "{msg}");
        assert!(msg.contains("you should try pattern 'refl'"), "{msg}");
    }

    #[test]
    fn refl_pattern_on_a_contradictory_equation() {
        let msg = error(&format!(
            "{NAT} let f : Id Nat zero (suc zero) -> Nat = \\ refl . zero; U"
        ));
        assert!(msg.contains("It should not be 'refl'"), "{msg}");
        assert!(msg.contains("you should try pattern '(!)'"), "{msg}");
    }

    #[test]
    fn undecidable_equations_are_reported_as_not_sure() {
        let src = format!(
            "{NAT_PLUS} let U2 : (m n : Nat) -> Id Nat (plus m zero) (plus n zero) -> Nat = \
\\ m n refl . zero; U"
        );
        let msg = error(&src);
        assert!(
            msg.starts_with("I am not sure if there should be a pattern 'refl'"),
            "{msg}"
        );
        assert!(
            msg.contains("plus m zero and plus n zero")
                || msg.contains("elim Nat"),
            "{msg}"
        );
    }

    #[test]
    fn too_many_patterns_are_reported() {
        assert_eq!(
            error(&format!("{NAT} let f : Nat -> Nat = \\ x y . x; U")),
            "Too many patterns."
        );
    }

    #[test]
    fn errors_point_at_the_offending_token() {
        let src = format!("{NAT}\nlet bad : Nat = U; U");
        assert_eq!(error_pos(&src), (2, 17));
        assert_eq!(error_pos("x"), (1, 1));
        // second line
        assert_eq!(error_pos(&format!("{NAT}\nlet bad : Nat =\n  U; U")), (3, 3));
    }

    #[test]
    fn definitions_can_be_applied_after_a_pattern_substitution() {
        // The original cannot do this (`Elab.hs` matches on the inferred type
        // without forcing it first); the port forces.  See NOTES.md.
        let src = format!(
            "{NAT} let go : (A : U) (a : A) -> Nat = \\ A a . zero; \
let use : (A : U) (a : A) -> Id A a a -> Nat = \\ A a refl . go A a; U"
        );
        assert_eq!(normal_form(&src), "U");
    }

    // -- parser errors -----------------------------------------------------

    #[test]
    fn a_forced_domain_is_matched_after_a_refinement() {
        // Regression: `check_pm` used to match the Pi domain without forcing
        // it, so a domain that became a delayed substitution after an earlier
        // `refl` pattern was rejected as "not an identity type".
        let src = format!(
            "{NAT} let A : U = Id Nat zero zero; \
let F : (x : Nat) -> Id Nat x x -> A -> Nat = \\ x refl refl . zero; U"
        );
        assert_eq!(normal_form(&src), "U");
    }

    #[test]
    fn printed_numerals_can_be_read_back() {
        // Constructors print under the names they were declared with, so
        // normal forms round-trip.
        let printed = normal_form(&format!("{NAT_PLUS} plus (suc zero) (suc zero)"));
        assert_eq!(printed, "suc (suc zero)");
        assert!(typecheck(&format!("{NAT} let z : Nat = {printed}; z")).is_ok());
    }

    #[test]
    fn freshened_names_can_be_read_back() {
        // Freshened names contain an apostrophe, so identifiers accept one.
        assert!(typecheck("let z : U = (A : U) -> (A' : U) -> A'; z").is_ok());
        assert!(typecheck("let z : U = (A' : U) -> A'; z").is_ok());
        assert!(typecheck("let z : U = (A : U) -> (A' : U) -> A; z").is_ok());
        assert_eq!(type_of("(A : U) -> (A : U) -> A"), "U");
    }

    #[test]
    fn unterminated_block_comments_are_rejected() {
        assert!(typecheck("U {- oops").is_err());
        assert!(typecheck("U {- oops -}").is_ok());
    }

    #[test]
    fn keywords_may_appear_inside_identifiers() {
        // Backtracking means `letx` is an identifier; the original commits on
        // the `let` prefix and rejects it (see NOTES.md).  The former
        // built-ins are ordinary identifiers now.
        assert_eq!(
            normal_form(&format!("{NAT} let letx : Nat = zero; letx")),
            "zero"
        );
        assert_eq!(error("sucx"), "unbound variable: sucx");
        assert_eq!(error(&format!("{NAT} plusx")), "unbound variable: plusx");
    }

    #[test]
    fn parse_errors_are_reported_with_positions() {
        let e = typecheck(&format!("{NAT_PLUS} Id Nat zero")).unwrap_err();
        assert_eq!(e.pos.line, 1);
        assert!(e.pos.col > 1);
        let e = typecheck("(").unwrap_err();
        assert_eq!(e.pos.file.to_string(), "<stdin>");
        assert_eq!(e.msg, "expected an atom");
    }


    // -- structs (sugar over single-constructor declarations) ---------------

    const PAIR: &str = "struct Pair (A : U) (B : U) : U where fst : A; snd : B;";

    #[test]
    fn structs_desugar_to_single_constructor_declarations() {
        let src = format!("{NAT} {PAIR} Pair Nat Nat");
        assert_eq!(type_of(&src), "U");
        // the generated constructor and projections are ordinary bindings
        let src = format!("{NAT} {PAIR} mkPair Nat Nat zero (suc zero)");
        assert_eq!(type_of(&src), "Pair Nat Nat");
        let src = format!(
            "{NAT} {PAIR} \
let p : Pair Nat Nat = mkPair Nat Nat (suc zero) (suc (suc zero)); \
suc (p.snd)"
        );
        assert_eq!(normal_form(&src), "suc (suc (suc zero))");
    }

    #[test]
    fn projections_work_generically_and_reduce() {
        let src = format!(
            "{NAT} {PAIR} \
let swap : (A : U) (B : U) -> Pair A B -> Pair B A = \
  λ A B p . mkPair B A (p.snd) (p.fst); \
(swap Nat Nat (mkPair Nat Nat zero (suc zero))).snd"
        );
        // the swapped second component is the original first one
        assert_eq!(normal_form(&src), "zero");
        // projections are functions too (the generated `fst`)
        let src = format!(
            "{NAT} {PAIR} \
fst Nat Nat (mkPair Nat Nat (suc zero) zero)"
        );
        assert_eq!(normal_form(&src), "suc zero");
    }

    #[test]
    fn projections_chain_and_apply() {
        let src = format!(
            "{NAT_PLUS} {PAIR} \
struct Point : U where px : Nat; py : Nat; \
struct Line : U where start : Point; end : Point; \
let a : Point = mkPoint zero (suc zero); \
let b : Point = mkPoint (suc (suc zero)) zero; \
let l : Line = mkLine a b; \
plus (l.start.py) (l.end.px)"
        );
        assert_eq!(normal_form(&src), "suc (suc (suc zero))");
    }

    #[test]
    fn zero_field_structs_and_elim_still_work() {
        // a zero-field struct is a singleton
        assert_eq!(
            normal_form("struct Unit1 : U where mkUnit1"),
            "mkUnit1"
        );
        // elim works on the desugared declaration
        assert_eq!(
            normal_form(&format!(
                "{NAT} {PAIR} \
let proj2 : (A : U) (B : U) -> Pair A B -> B = \
  λ A B p . elim Pair A B (λ _ . B) (λ x y . y) p; \
proj2 Nat Nat (mkPair Nat Nat zero (suc zero))"
            )),
            "suc zero"
        );
    }

    #[test]
    fn struct_fields_cannot_depend_on_earlier_fields() {
        // A dependent field type makes the generated projection ill-typed
        // (the earlier field is only in scope inside the constructor, not in
        // the projection's type), so the declaration itself is rejected.
        let msg = error("struct Dep : U where carrier : U; elem : carrier; U");
        assert!(
            msg.contains("type mismatch") || msg.contains("unbound variable"),
            "{msg}"
        );
        let msg = error(&format!(
            "{NAT} struct P : U where n : Nat; m : n; let p : P = mkP zero zero; p.m"
        ));
        assert!(
            msg.contains("type mismatch")
                || msg.contains("unbound variable")
                || msg.contains("may not depend on an earlier field"),
            "{msg}"
        );
    }

    #[test]
    fn projections_are_only_for_single_constructor_types() {
        let msg = error(&format!(
            "{NAT} data Nat : U where zero : Nat; suc : (n : Nat) -> Nat; zero.suc"
        ));
        // zero : Nat, but Nat has two constructors
        assert!(
            msg.contains("more than one constructor") || msg.contains("no field"),
            "{msg}"
        );
        let msg = error(&format!("{NAT} {PAIR} let p : Nat = zero; p.fst"));
        assert!(
            msg.contains("more than one constructor")
                || msg.contains("is not a declared type")
                || msg.contains("needs a struct"),
            "{msg}"
        );
    }

    // -- modes -------------------------------------------------------------

    #[test]
    fn run_prints_normal_form_and_type() {
        assert_eq!(run("U", Mode::Nf).unwrap(), "U\n  :\nU");
        assert_eq!(
            run(
                &format!("{NAT_PLUS} plus (suc zero) (suc zero)"),
                Mode::Nf
            )
            .unwrap(),
            "suc (suc zero)\n  :\nNat"
        );
        assert_eq!(
            run(&format!("{NAT_PLUS} plus zero zero"), Mode::Type).unwrap(),
            "Nat"
        );
    }

    #[test]
    fn run_reports_errors() {
        let e = run("refl", Mode::Nf).unwrap_err();
        assert_eq!(e.msg, "Can't infer type for refl");
        assert!(e.render("refl").contains("<stdin>:1:1:"));
    }

    #[test]
    fn deep_terms_run_on_a_large_stack() {
        // The pipeline recurses over the program structure, so library users
        // should use a thread with a generous stack -- exactly what the CLI
        // does.  `parse::MAX_DEPTH` bounds how deep a program can be; this
        // stays just below the bound.
        let mut src = "zero".to_string();
        for _ in 0..150 {
            src = format!("suc ({src})");
        }
        let src = format!("{NAT} let x : Nat = {src}; x");
        let expected = printed_nat(150);

        let printed = std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                normalize(&src)
                    .map(|t| pretty::show_tm(&t))
                    .map_err(|e| e.render(&src))
            })
            .expect("thread spawns")
            .join()
            .expect("the worker thread must not panic")
            .expect("the program typechecks");
        assert_eq!(printed, expected);
    }

    #[test]
    fn absurd_pattern_lambdas_have_a_value() {
        // DEVIATION 7: the original panics (`eval: impossible (Bot)`) as soon
        // as an absurd-pattern lambda is evaluated.
        let src = format!(
            "{NAT} let C : (A : U) -> Id Nat (suc zero) zero -> A = \\ A (!); C"
        );
        assert_eq!(normal_form(&src), "λ A (!) . ⊥");
        assert_eq!(type_of(&src), "(A : U) → Id Nat (suc zero) zero → A");
    }
}
