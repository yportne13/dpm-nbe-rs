//! Black-box golden tests for the `dpm-nbe` binary.
//!
//! Every expected value below was derived by running the binary and reviewed
//! by hand.  Programs that use arithmetic start with the same three-line
//! prelude (a `data Nat` declaration and a `plus` defined through the
//! generated eliminator), so error positions on later lines stay stable.
//!
//! ```text
//! data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;
//! let plus : Nat -> Nat -> Nat =
//!   λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;
//! ```
//!
//! Cases whose name ends in `_DEVIATION` encode port behaviour documented in
//! `../NOTES.md` (printable lambdas, saturating quotation, `Bot` as a value,
//! `(!)` in messages, stuck-eliminator conversion, stderr error reporting).
//!
//! Only std is used; the binary is located through `CARGO_BIN_EXE_dpm-nbe`.

use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_dpm-nbe");

/// The arithmetic prelude shared by the Nat-using programs.
const PRELUDE: &str = "data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;\nlet plus : Nat -> Nat -> Nat =\n  λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;\n";

/// The whole program from README.md, wrapped as `... ; U`.
const README_FULL: &str = include_str!("fixtures/readme_full.txt");

/// (name, mode, source-after-prelude, expected stdout) for exit-code-0 runs.
static OK: &[(&str, &str, &str, &str)] = &[
    (
        "readme_sym",
        "nf",
        "let sym : (A : U) (a b : A) -> Id A a b -> Id A b a = λ A a b refl . refl; U",
        "U\n  :\nU1\n",
    ),
    (
        "readme_trans",
        "nf",
        "let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl . refl; U",
        "U\n  :\nU1\n",
    ),
    (
        "readme_cong",
        "nf",
        "let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) = λ A B f a b refl . refl; U",
        "U\n  :\nU1\n",
    ),
    (
        "readme_H",
        "nf",
        "let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n refl refl . refl; U",
        "U\n  :\nU1\n",
    ),
    (
        "readme_C_absurd",
        "nf",
        "let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); U",
        "U\n  :\nU1\n",
    ),
    (
        "nf_C_absurd_value_DEVIATION",
        "nf",
        "let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C",
        "λ A (!) . ⊥\n  :\n(A : U) → Id Nat (suc zero) zero → A\n",
    ),
    (
        "nf_plus_ss",
        "nf",
        "plus (suc zero) (suc zero)",
        "suc (suc zero)\n  :\nNat\n",
    ),
    (
        "nf_plus_s2_z",
        "nf",
        "plus (suc (suc zero)) zero",
        "suc (suc zero)\n  :\nNat\n",
    ),
    (
        "nf_plus_s_z",
        "nf",
        "plus (suc zero) zero",
        "suc zero\n  :\nNat\n",
    ),
    (
        "nf_plus_zz",
        "nf",
        "plus zero zero",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_let_plus_xx",
        "nf",
        "let x : Nat = zero; plus x x",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_let_plus_x_sx",
        "nf",
        "let x : Nat = suc zero; plus x x",
        "suc (suc zero)\n  :\nNat\n",
    ),
    (
        "nf_suc_zero",
        "nf",
        "suc zero",
        "suc zero\n  :\nNat\n",
    ),
    (
        "nf_suc_suc_zero",
        "nf",
        "suc (suc zero)",
        "suc (suc zero)\n  :\nNat\n",
    ),
    (
        "nf_zero",
        "nf",
        "zero",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_U",
        "nf",
        "U",
        "U\n  :\nU1\n",
    ),
    (
        "nf_let_id_applied",
        "nf",
        "let f : Nat -> Nat = λ x . x; f (suc zero)",
        "suc zero\n  :\nNat\n",
    ),
    (
        "nf_let_id_zero",
        "nf",
        "let f : Nat -> Nat = λ x . x; f zero",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_backslash_lambda",
        "nf",
        "let f : Nat -> Nat = \\ x . x; f zero",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_arrow_is_a_type",
        "nf",
        "Nat -> Nat",
        "Nat → Nat\n  :\nU\n",
    ),
    (
        "nf_unicode_arrow",
        "nf",
        "let f : Nat → Nat = λ x . x; f (suc zero)",
        "suc zero\n  :\nNat\n",
    ),
    (
        "nf_anonymous_pi_body",
        "nf",
        "(A : U) -> A -> A",
        "(A : U) → A → A\n  :\nU1\n",
    ),
    (
        "nf_poly_identity_names_DEVIATION",
        "nf",
        "let f : (A : U) -> A -> A = λ A x . x; f",
        "λ A x . x\n  :\n(A : U) → A → A\n",
    ),
    (
        "nf_stuck_plus_right",
        "nf",
        "let f : Nat -> Nat = λ n . plus n zero; f",
        "λ n . elim Nat (λ _ . Nat) zero (λ _ r . suc r) n\n  :\nNat → Nat\n",
    ),
    (
        "nf_plus_zero_left_is_id",
        "nf",
        "let f : Nat -> Nat = λ n . plus zero n; f",
        "λ n . n\n  :\nNat → Nat\n",
    ),
    (
        "nf_partial_application_of_lambda_DEVIATION",
        "nf",
        "let k : Nat -> Nat -> Nat = λ x y . plus x y; k zero",
        "λ y . y\n  :\nNat → Nat\n",
    ),
    (
        "nf_partial_application_saturated_suc_DEVIATION",
        "nf",
        "let k : Nat -> Nat -> Nat = λ x y . plus x y; k (suc zero)",
        "λ y . suc y\n  :\nNat → Nat\n",
    ),
    (
        "nf_refl_plus_neutral_reflexive_DEVIATION",
        "nf",
        "let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\ n m . refl; f",
        "λ n m . refl\n  :\n(n : Nat)(m : Nat) → Id Nat (elim Nat (λ _ . Nat) m (λ _ r . suc r) n) (elim Nat (λ _ . Nat) m (λ _ r . suc r) n)\n",
    ),
    (
        "nf_id_nat_zero_zero",
        "nf",
        "Id Nat zero zero",
        "Id Nat zero zero\n  :\nU\n",
    ),
    (
        "nf_id_nat_suc_zero_zero",
        "nf",
        "Id Nat (suc zero) zero",
        "Id Nat (suc zero) zero\n  :\nU\n",
    ),
    (
        "nf_id_U_nat_nat",
        "nf",
        "Id U Nat Nat",
        "Id U Nat Nat\n  :\nU1\n",
    ),
    (
        "nf_let_type_alias",
        "nf",
        "let A : U = Nat; A",
        "Nat\n  :\nU\n",
    ),
    (
        "nf_nested_let",
        "nf",
        "let a : Nat = zero; let b : Nat = a; b",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_let_chain_plus",
        "nf",
        "let x : Nat = suc zero; let y : Nat = x; plus y y",
        "suc (suc zero)\n  :\nNat\n",
    ),
    (
        "nf_shadowing_let",
        "nf",
        "let x : Nat = zero; let x : Nat = suc x; x",
        "suc zero\n  :\nNat\n",
    ),
    (
        "nf_lam_two_args",
        "nf",
        "let g : Nat -> Nat -> Nat = λ x y . plus x y; g",
        "λ x y . elim Nat (λ _ . Nat) y (λ _ r . suc r) x\n  :\nNat → Nat → Nat\n",
    ),
    (
        "nf_named_dependent_pi",
        "nf",
        "(n : Nat) -> Id Nat n n",
        "(n : Nat) → Id Nat n n\n  :\nU\n",
    ),
    (
        "nf_two_named_binders",
        "nf",
        "(m : Nat) (n : Nat) -> Nat",
        "(m : Nat)(n : Nat) → Nat\n  :\nU\n",
    ),
    (
        "nf_let_refl_value",
        "nf",
        "let p : Id Nat zero zero = refl; p",
        "refl\n  :\nId Nat zero zero\n",
    ),
    (
        "nf_let_poly_const_applied",
        "nf",
        "let f : (A : U) -> A -> Nat = λ A x . zero; f Nat zero",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_plus_nested",
        "nf",
        "plus (plus (suc zero) zero) (suc zero)",
        "suc (suc zero)\n  :\nNat\n",
    ),
    (
        "nf_dependent_function",
        "nf",
        "let f : (m : Nat) -> Id Nat m (plus m zero) -> Nat = λ m p . zero; f",
        "λ m p . zero\n  :\n(m : Nat) → Id Nat m (elim Nat (λ _ . Nat) zero (λ _ r . suc r) m) → Nat\n",
    ),
    (
        "nf_H_value",
        "nf",
        "let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n refl refl . refl; H",
        "λ m n refl refl . refl\n  :\n(m : Nat)(n : Nat) → Id Nat n (elim Nat (λ _ . Nat) zero (λ _ r . suc r) m) → Id Nat m zero → Id Nat n zero\n",
    ),
    (
        "nf_trans_value",
        "nf",
        "let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl . refl; trans",
        "λ A a b c refl refl . refl\n  :\n(A : U)(a : A)(b : A)(c : A) → Id A a b → Id A b c → Id A a c\n",
    ),
    (
        "nf_sym_value",
        "nf",
        "let sym : (A : U) (a b : A) -> Id A a b -> Id A b a = λ A a b refl . refl; sym",
        "λ A a b refl . refl\n  :\n(A : U)(a : A)(b : A) → Id A a b → Id A b a\n",
    ),
    (
        "nf_cong_value",
        "nf",
        "let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) = λ A B f a b refl . refl; cong",
        "λ A B f a b refl . refl\n  :\n(A : U)(B : U)(f : A → B)(a : A)(b : A) → Id A a b → Id B (f a) (f b)\n",
    ),
    (
        "nf_line_comment",
        "nf",
        "let x : Nat = zero; -- comment\nplus x x",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_block_comment",
        "nf",
        "let x : Nat = {- hi -} suc zero; plus x zero",
        "suc zero\n  :\nNat\n",
    ),
    (
        "nf_data_declares_a_type",
        "nf",
        "data Bool : U where true : Bool; false : Bool; Bool",
        "Bool\n  :\nU\n",
    ),
    (
        "nf_bool_elim_not_not",
        "nf",
        "data Bool : U where true : Bool; false : Bool; let not : Bool -> Bool = λ b . elim Bool (λ _ . Bool) false true b; not (not true)",
        "true\n  :\nBool\n",
    ),
    (
        "nf_elim_head_is_the_recursor",
        "nf",
        "elim Nat",
        "elim Nat\n  :\n(P : Nat → U) → P zero → ((n : Nat)(n' : P n) → P (suc n)) → (x : Nat) → P x\n",
    ),
    (
        "nf_list_elim_head",
        "nf",
        "data List (A : U) : U where nil : List A; cons : (a : A) -> (as : List A) -> List A; elim List",
        "elim List\n  :\n(A : U)(P : List A → U) → P (nil A) → ((a : A)(as : List A)(as' : P as) → P (cons A a as)) → (x : List A) → P x\n",
    ),
    (
        "type_Nat",
        "type",
        "Nat",
        "U\n",
    ),
    (
        "type_zero",
        "type",
        "zero",
        "Nat\n",
    ),
    (
        "type_suc_zero",
        "type",
        "suc zero",
        "Nat\n",
    ),
    (
        "type_suc_suc_zero",
        "type",
        "suc (suc zero)",
        "Nat\n",
    ),
    (
        "type_plus_zz",
        "type",
        "plus zero zero",
        "Nat\n",
    ),
    (
        "type_id_nat_zz",
        "type",
        "Id Nat zero zero",
        "U\n",
    ),
    (
        "type_id_U_nat_nat",
        "type",
        "Id U Nat Nat",
        "U1\n",
    ),
    (
        "type_pi_anonymous",
        "type",
        "(A : U) -> A -> A",
        "U1\n",
    ),
    (
        "type_U",
        "type",
        "U",
        "U1\n",
    ),
    (
        "type_arrow",
        "type",
        "Nat -> Nat",
        "U\n",
    ),
    (
        "type_unicode_arrow",
        "type",
        "Nat → Nat",
        "U\n",
    ),
    (
        "type_named_pi",
        "type",
        "(n : Nat) -> Id Nat n n",
        "U\n",
    ),
    (
        "type_let_id",
        "type",
        "let f : Nat -> Nat = λ x . x; f",
        "Nat → Nat\n",
    ),
    (
        "type_let_dependent",
        "type",
        "let f : (m : Nat) -> Id Nat m (plus m zero) -> Nat = λ m p . zero; f",
        "(m : Nat) → Id Nat m (elim Nat (λ _ . Nat) zero (λ _ r . suc r) m) → Nat\n",
    ),
    (
        "type_let_applied",
        "type",
        "let f : Nat -> Nat = λ x . x; f zero",
        "Nat\n",
    ),
    (
        "type_let_var",
        "type",
        "let x : Nat = zero; x",
        "Nat\n",
    ),
    (
        "type_let_nested",
        "type",
        "let a : Nat = zero; let b : Nat = a; b",
        "Nat\n",
    ),
    // -- regression rows for the inductive-types fixes ----------------------
    //
    // A constructor argument that merely *ends* in the datatype (`Nat -> R`,
    // `U -> R`) is not recursive: its method binds the argument and nothing
    // else, so `elim R` below has a one-argument mk-method.  The first row
    // uses a `U -> R` argument, which lives in `U1`, so `R` itself is
    // declared there (the ctor-arg universe guard rejects it at `U`); the
    // second keeps the same semantics with a domain small enough for any
    // universe, and the third pins the generated method type.
    (
        "nf_elim_fn_arg_no_ih",
        "nf",
        "data R : U1 where base : R; mk : (U -> R) -> R;\nelim R (\\ _ . Nat) zero (\\ f . zero) (mk (\\ u . base))",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_elim_fn_arg_no_ih_nat_domain",
        "nf",
        "data R : U where base : R; mk : (Nat -> R) -> R;\nelim R (\\ _ . Nat) zero (\\ f . zero) (mk (\\ n . base))",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_elim_head_fn_arg_no_ih",
        "nf",
        "data R : U where base : R; mk : (Nat -> R) -> R; elim R",
        "elim R\n  :\n(P : R → U) → P base → ((Nat → R) → P (mk _)) → (x : R) → P x\n",
    ),
    // A datatype may store small-universe codes only if it is declared one
    // level up (`Wrap1`/`Wrap2` in `U1`), while the `U`-level `Wrap` below is
    // rejected (see `err_ctor_arg_from_higher_universe`).
    (
        "nf_data_high_universe_stores_small_code",
        "nf",
        "data Wrap1 : U1 where mk : U -> Wrap1; mk Nat",
        "mk Nat\n  :\nWrap1\n",
    ),
    (
        "nf_struct_high_universe_stores_small_code",
        "nf",
        "struct Wrap2 : U1 where carrier : U; mkWrap2 Nat",
        "mkWrap2 Nat\n  :\nWrap2\n",
    ),
    // A struct field may mention the struct itself; the projections then
    // typecheck (no closed value of such a struct exists without recursion).
    (
        "nf_struct_recursive_field_hd",
        "nf",
        "struct Box : U where n : Nat; b : Box; let hd : Box -> Nat = \\ b . b.n; U",
        "U\n  :\nU1\n",
    ),
    (
        "nf_struct_recursive_field_tl",
        "nf",
        "struct Box : U where n : Nat; b : Box; let tl : Box -> Box = \\ b . b.b; U",
        "U\n  :\nU1\n",
    ),
    // The generated projection `fst` and the surface projection `p.fst` are
    // the same function: applied to one pair they must both reduce to the
    // same numeral, so `plus` of the two is `2`.
    (
        "nf_struct_projection_forms_agree",
        "nf",
        "struct Pair (A : U) (B : U) : U where fst : A; snd : B; let p : Pair Nat Nat = mkPair Nat Nat (suc zero) zero; plus (fst Nat Nat p) (p.fst)",
        "suc (suc zero)\n  :\nNat\n",
    ),
    // A struct parameter may be called `p` (the name the desugaring used to
    // use for its own binder) without capturing anything.
    (
        "nf_struct_param_named_p",
        "nf",
        "struct Foo (p : U) : U where f : p; let x : Foo Nat = mkFoo Nat zero; x.f",
        "zero\n  :\nNat\n",
    ),
    // The parser accepts universe levels up to 2^30 - 1; the level's own
    // type is the next universe up.
    (
        "nf_U_max_level",
        "nf",
        "U1073741823",
        "U1073741823\n  :\nU1073741824\n",
    ),
    // -- holes and pruning (NOTES.md deviation 18) --------------------------
    //
    // A hole in a let annotation is pinned by the checked value; one in a Pi
    // codomain picks up the binder as its spine; `refl` solves a hole in an
    // identity type; and inferring `(\ f . f zero) suc` refines `f`'s type
    // meta to a function type -- pruning `f` off the refinement metas --
    // before the argument solves domain and codomain.
    (
        "nf_hole_in_let_annotation",
        "nf",
        "let x : _ = zero; suc x",
        "suc zero\n  :\nNat\n",
    ),
    (
        "nf_lambda_meta_domain_pruning",
        "nf",
        "(\\ f . f zero) suc",
        "suc zero\n  :\nNat\n",
    ),
    (
        "type_hole_in_pi_codomain",
        "type",
        "let f : Nat -> _ = \\ n . zero; f zero",
        "Nat\n",
    ),
    (
        "nf_refl_solves_hole_in_id",
        "nf",
        "let p : Id Nat _ zero = refl; p",
        "refl\n  :\nId Nat zero zero\n",
    ),
];

/// (name, mode, source-after-prelude, line, col, message) for exit-code-1
/// elaboration errors.  Lines count from the start of the whole program.
static ERR: &[(&str, &str, &str, usize, usize, &str)] = &[
    (
        "err_lambda_nf",
        "nf",
        "λ x . x",
        4,
        1,
        "unsolved metavariable ?0\n",
    ),
    (
        "err_lambda_pattern_nf",
        "nf",
        "λ x refl . x",
        4,
        1,
        "Can't infer type for lambda expression\n",
    ),
    (
        "err_unbound_x",
        "nf",
        "x",
        4,
        1,
        "unbound variable: x\n",
    ),
    (
        "err_unbound_z_in_let",
        "nf",
        "let y : Nat = z; y",
        4,
        15,
        "unbound variable: z\n",
    ),
    (
        "err_mismatch_U_against_Nat",
        "nf",
        "let x : Nat = U; x",
        4,
        15,
        "type mismatch\n\nexpected type:\n\n  Nat\n\ninferred type:\n\n  U1\n\n",
    ),
    (
        "err_mismatch_suc_arg",
        "nf",
        "suc U",
        4,
        5,
        "type mismatch\n\nexpected type:\n\n  Nat\n\ninferred type:\n\n  U1\n\n",
    ),
    (
        "err_not_a_function",
        "nf",
        "zero zero",
        4,
        1,
        "Expected a function type, instead inferred:\n\n  Nat\n",
    ),
    (
        "err_refl_pattern_on_nat",
        "nf",
        "let f : Nat -> Nat = λ refl . zero; U",
        4,
        22,
        "Expected an identity type for pattern 'refl', but got:\n\n  Nat\n",
    ),
    (
        "err_abs_pattern_on_nat",
        "nf",
        "let f : Nat -> Nat -> Nat = λ x (!); U",
        4,
        29,
        "Expected an identity type for pattern '(!)', but got:\n\n  Nat\n",
    ),
    (
        "err_too_many_patterns",
        "nf",
        "let f : Nat = λ x . zero; U",
        4,
        15,
        "Too many patterns.\n",
    ),
    (
        "err_refl_not_sure",
        "nf",
        "let f : (m : Nat) -> Id Nat (plus m zero) (suc zero) -> Nat = λ m refl . zero; U",
        4,
        63,
        "I am not sure if there should be a pattern 'refl' for equation between elim Nat (λ _ . Nat) zero (λ _ r . suc r) m and suc zero.\n",
    ),
    (
        "err_refl_should_be_absurd",
        "nf",
        "let f : Id Nat zero (suc zero) -> Nat = λ refl . zero; U",
        4,
        41,
        "It should not be 'refl' for equation between zero and suc zero, in fact you should try pattern '(!)'.\n",
    ),
    (
        "err_absurd_should_be_refl",
        "nf",
        "let f : (x : Nat) -> Id Nat zero zero -> Nat = λ x (!); U",
        4,
        48,
        "It should not be '(!)' for equation between zero and zero, in fact you should try pattern 'refl'.\n",
    ),
    (
        "err_second_line_not_a_function",
        "nf",
        "let x : Nat = zero;\nzero zero",
        5,
        1,
        "Expected a function type, instead inferred:\n\n  Nat\n",
    ),
    (
        "err_second_line_mismatch",
        "nf",
        "let x : Nat = zero;\nsuc U",
        5,
        5,
        "type mismatch\n\nexpected type:\n\n  Nat\n\ninferred type:\n\n  U1\n\n",
    ),
    (
        "err_ctor_result_must_be_the_datatype",
        "nf",
        "data Bad : U where mk : U; U",
        4,
        1,
        "constructor mk must return Bad\n",
    ),
    (
        "err_not_strictly_positive",
        "nf",
        "data Bad : U where mk : (Bad -> U) -> Bad; U",
        4,
        1,
        "constructor mk of Bad is not strictly positive: Bad may only occur as the whole type of an argument, or as the result of a function type\n",
    ),
    (
        "err_elim_expects_a_data_type",
        "nf",
        "elim U",
        4,
        1,
        "elim expects a data type\n",
    ),
    // -- regression rows for the inductive-types fixes ----------------------
    //
    // The mk-method for a function-typed argument takes exactly one argument,
    // so a two-pattern method is one pattern too many.
    (
        "err_elim_fn_arg_method_arity",
        "nf",
        "data R : U where base : R; mk : (Nat -> R) -> R;\nelim R (\\ _ . Nat) zero (\\ f r . r) (mk (\\ n . base))",
        5,
        26,
        "Too many patterns.\n",
    ),
    // A `U`-level datatype may not store a small-universe code.
    (
        "err_ctor_arg_from_higher_universe",
        "nf",
        "data Wrap : U where mk : U -> Wrap; Wrap",
        4,
        1,
        "constructor mk of Wrap may not store arguments from a higher universe (declare Wrap in a higher universe)\n",
    ),
    // Two fields of one struct may not share a name.
    (
        "err_struct_duplicate_field",
        "nf",
        "struct S : U where a : Nat; a : Nat; U",
        4,
        1,
        "struct S has duplicate field a\n",
    ),
];

/// (name, source) garbage that must not parse.
static PARSE_ERR: &[(&str, &str)] = &[
    ("parse_elim_partial", "elim"),
    ("parse_data_no_name", "data : U where U"),
    ("parse_data_missing_where", "data Nat U"),
    ("parse_ctor_missing_semi", "data Bad : U where mk : U U"),
    ("parse_id_partial", "Id Nat zero"),
    ("parse_let_missing_colon", "let x = zero; x"),
    ("parse_lam_no_pattern", "λ . refl"),
    ("parse_garbage", "@"),
    ("parse_unbalanced_paren", "(A : U -> A"),
    ("parse_stray_rparen", "Id Nat zero zero)"),
    ("parse_missing_body", "let x : Nat = zero;"),
    ("parse_empty_input", ""),
    // The largest accepted universe level is 2^30 - 1; one more does not
    // parse (see also `nf_U_max_level` above).
    ("parse_U_level_too_large", "U1073741824"),
];

// ---------------------------------------------------------------------------
// harness
// ---------------------------------------------------------------------------

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str], stdin: &str) -> Run {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn {BIN}: {e}"));
    // Taking the handle drops (closes) stdin after the write, giving the child EOF.
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(stdin.as_bytes())
        .expect("write to child stdin");
    let out = child.wait_with_output().expect("wait for child");
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn indent(s: &str) -> String {
    s.lines()
        .map(|l| format!("      {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Mirror of `genTransN` in Main.hs; `tail` is the final expression (`U`, or
/// `transN` to observe the elaborated value/type).
fn trans_prog(n: usize, tail: &str) -> String {
    let mut s = format!("let trans{n} : (A : U) ");
    for i in 1..=n {
        s.push_str(&format!("(a{i} : A) "));
    }
    for i in 1..n {
        s.push_str(&format!("-> Id A a{i} a{} ", i + 1));
    }
    s.push_str(&format!("-> Id A a1 a{n} = "));
    s.push_str("\u{3bb} A ");
    for i in 1..=n {
        s.push_str(&format!("a{i} "));
    }
    for _ in 1..n {
        s.push_str("refl ");
    }
    s.push_str(&format!(". refl; {tail}"));
    s
}

/// Expected normal form of `genTransN n` when the value is observed.
fn trans_nf(n: usize) -> String {
    let mut s = String::from("\u{3bb} A");
    for i in 1..=n {
        s.push_str(&format!(" a{i}"));
    }
    for _ in 1..n {
        s.push_str(" refl");
    }
    s.push_str(" . refl");
    s
}

/// Expected type of `genTransN n`, as the printer renders it (consecutive
/// named binders without a separator).
fn trans_ty(n: usize) -> String {
    let mut s = String::from("(A : U)");
    for i in 1..=n {
        s.push_str(&format!("(a{i} : A)"));
    }
    for i in 1..n {
        s.push_str(&format!(" \u{2192} Id A a{i} a{}", i + 1));
    }
    s.push_str(&format!(" \u{2192} Id A a1 a{n}"));
    s
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[test]
fn golden_success_cases() {
    let mut cases: Vec<(String, String, String, String)> = OK
        .iter()
        .map(|&(n, m, s, e)| {
            (
                n.to_string(),
                m.to_string(),
                format!("{PRELUDE}{s}\n"),
                e.to_string(),
            )
        })
        .collect();

    // The README program carries its own declarations.
    cases.push((
        "readme_full_program".to_string(),
        "nf".to_string(),
        README_FULL.to_string(),
        "U\n  :\nU1\n".to_string(),
    ));

    // `genTransN` family from Main.hs, n = 1..=6.  As written in Main.hs the
    // program ends in `U`, so the whole program normalises to `U : U`.
    for n in 1..=6usize {
        cases.push((
            format!("gen_trans_{n}"),
            "nf".to_string(),
            trans_prog(n, "U"),
            "U\n  :\nU1\n".to_string(),
        ));
    }
    // ... and the same definition observed through a reference to it, which
    // exercises the normal form and the inferred type of the generated term.
    for n in 2..=6usize {
        cases.push((
            format!("gen_trans_value_{n}"),
            "nf".to_string(),
            trans_prog(n, &format!("trans{n}")),
            format!("{}\n  :\n{}\n", trans_nf(n), trans_ty(n)),
        ));
    }

    assert!(
        cases.len() >= 30,
        "the case table must keep at least 30 cases, found {}",
        cases.len()
    );

    let mut failures: Vec<String> = Vec::new();
    for (name, mode, src, expected) in &cases {
        let out = run(&[mode.as_str()], src);
        if out.code != Some(0) {
            failures.push(format!(
                "[{name}] exit code {:?}, expected 0\n    src: {}\n    stderr:\n{}",
                out.code,
                esc(src),
                indent(&out.stderr)
            ));
            continue;
        }
        if out.stdout != *expected {
            failures.push(format!(
                "[{name}] stdout mismatch\n    src:      {}\n    expected: {}\n    actual:   {}",
                esc(src),
                esc(expected),
                esc(&out.stdout)
            ));
        }
        if !out.stderr.is_empty() {
            failures.push(format!(
                "[{name}] expected empty stderr on success\n    stderr:\n{}",
                indent(&out.stderr)
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} success case(s) failed:\n\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n\n")
    );
}

/// Literal cross-check of the generated `transN` family against hand-derived text.
#[test]
fn gen_trans_literals() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "trans1_value",
            "let trans1 : (A : U) (a1 : A) -> Id A a1 a1 = \u{3bb} A a1 . refl; trans1",
            "\u{3bb} A a1 . refl\n  :\n(A : U)(a1 : A) \u{2192} Id A a1 a1\n",
        ),
        (
            "trans3_value",
            "let trans3 : (A : U) (a1 : A) (a2 : A) (a3 : A) -> Id A a1 a2 -> Id A a2 a3 -> Id A a1 a3 = \u{3bb} A a1 a2 a3 refl refl . refl; trans3",
            "\u{3bb} A a1 a2 a3 refl refl . refl\n  :\n(A : U)(a1 : A)(a2 : A)(a3 : A) \u{2192} Id A a1 a2 \u{2192} Id A a2 a3 \u{2192} Id A a1 a3\n",
        ),
        (
            "trans6_value",
            "let trans6 : (A : U) (a1 : A) (a2 : A) (a3 : A) (a4 : A) (a5 : A) (a6 : A) -> Id A a1 a2 -> Id A a2 a3 -> Id A a3 a4 -> Id A a4 a5 -> Id A a5 a6 -> Id A a1 a6 = \u{3bb} A a1 a2 a3 a4 a5 a6 refl refl refl refl refl . refl; trans6",
            "\u{3bb} A a1 a2 a3 a4 a5 a6 refl refl refl refl refl . refl\n  :\n(A : U)(a1 : A)(a2 : A)(a3 : A)(a4 : A)(a5 : A)(a6 : A) \u{2192} Id A a1 a2 \u{2192} Id A a2 a3 \u{2192} Id A a3 a4 \u{2192} Id A a4 a5 \u{2192} Id A a5 a6 \u{2192} Id A a1 a6\n",
        ),
        (
            "trans1_program",
            "let trans1 : (A : U) (a1 : A) -> Id A a1 a1 = \u{3bb} A a1 . refl; U",
            "U\n  :\nU1\n",
        ),
        (
            "trans6_program",
            "let trans6 : (A : U) (a1 : A) (a2 : A) (a3 : A) (a4 : A) (a5 : A) (a6 : A) -> Id A a1 a2 -> Id A a2 a3 -> Id A a3 a4 -> Id A a4 a5 -> Id A a5 a6 -> Id A a1 a6 = \u{3bb} A a1 a2 a3 a4 a5 a6 refl refl refl refl refl . refl; U",
            "U\n  :\nU1\n",
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for &(name, src, expected) in cases {
        let out = run(&["nf"], src);
        if out.code != Some(0) || out.stdout != expected {
            failures.push(format!(
                "[{name}] exit {:?}\n    expected: {}\n    actual:   {}\n    stderr:\n{}",
                out.code,
                esc(expected),
                esc(&out.stdout),
                indent(&out.stderr)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn golden_error_cases() {
    assert!(
        ERR.len() >= 17,
        "the error table must keep all cases, found {}",
        ERR.len()
    );
    let mut failures: Vec<String> = Vec::new();

    for &(name, mode, body, line, col, msg) in ERR {
        let src = format!("{PRELUDE}{body}\n");
        let out = run(&[mode], &src);
        if out.code != Some(1) {
            failures.push(format!(
                "[{name}] exit code {:?}, expected 1\n    src: {}\n    stdout: {}\n    stderr:\n{}",
                out.code,
                esc(&src),
                esc(&out.stdout),
                indent(&out.stderr)
            ));
            continue;
        }
        if !out.stdout.is_empty() {
            failures.push(format!(
                "[{name}] expected empty stdout, got: {}",
                esc(&out.stdout)
            ));
        }

        let lines: Vec<&str> = out.stderr.lines().collect();
        let want_pos = format!("<stdin>:{line}:{col}:");
        if lines.first().copied() != Some(want_pos.as_str()) {
            failures.push(format!(
                "[{name}] first stderr line is {:?}, expected {:?}\n    src: {}\n    stderr:\n{}",
                lines.first(),
                want_pos,
                esc(&src),
                indent(&out.stderr)
            ));
            continue;
        }
        if lines.len() < 5 {
            failures.push(format!(
                "[{name}] stderr has only {} line(s), expected the 5-line displayError format\n    stderr:\n{}",
                lines.len(),
                indent(&out.stderr)
            ));
            continue;
        }

        let src_line = src.lines().nth(line - 1).unwrap_or("<missing>");
        let want_src = format!("{line} | {src_line}");
        if lines[2] != want_src {
            failures.push(format!(
                "[{name}] echoed source line is {:?}, expected {:?}",
                lines[2], want_src
            ));
        }
        // The rendering pins the padding line to `{pad} |` and the caret line
        // to `{pad} | {col - 1 spaces}^`, where `pad` is as wide as the line
        // number.
        let pad = " ".repeat(line.to_string().len());
        let want_pad_line = format!("{pad} |");
        if lines[1] != want_pad_line {
            failures.push(format!(
                "[{name}] padding line is {:?}, expected {:?}",
                lines[1], want_pad_line
            ));
        }
        let want_caret = format!("{pad} | {}^", " ".repeat(col - 1));
        if lines[3] != want_caret {
            failures.push(format!(
                "[{name}] caret line is {:?}, expected {:?} (caret under source column {col})",
                lines[3], want_caret
            ));
        }
        let got_msg = lines[4..].join("\n");
        if got_msg.trim_end() != msg.trim_end() {
            failures.push(format!(
                "[{name}] message mismatch\n    expected: {}\n    actual:   {}",
                esc(msg),
                esc(&got_msg)
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} error case(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn parse_errors_report_position_on_stderr() {
    assert!(
        PARSE_ERR.len() >= 10,
        "the parse error table must keep all cases, found {}",
        PARSE_ERR.len()
    );
    let mut failures: Vec<String> = Vec::new();

    for &(name, body) in PARSE_ERR {
        let src = format!("{PRELUDE}{body}\n");
        let out = run(&["nf"], &src);
        if out.code != Some(1) {
            failures.push(format!(
                "[{name}] exit code {:?}, expected 1\n    src: {}\n    stdout: {}\n    stderr:\n{}",
                out.code,
                esc(&src),
                esc(&out.stdout),
                indent(&out.stderr)
            ));
            continue;
        }
        if !out.stdout.is_empty() {
            failures.push(format!(
                "[{name}] parse error must not write to stdout, got: {}",
                esc(&out.stdout)
            ));
        }
        if out.stderr.trim().is_empty() {
            failures.push(format!("[{name}] expected an error message on stderr"));
            continue;
        }
        let first = out.stderr.lines().next().unwrap_or("");
        if position_of(first).is_none() {
            failures.push(format!(
                "[{name}] first stderr line is {:?}, expected a `<stdin>:line:col:` position\n    stderr:\n{}",
                first,
                indent(&out.stderr)
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} parse error case(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn position_of(line: &str) -> Option<(usize, usize)> {
    let rest = line.strip_prefix("<stdin>:")?.strip_suffix(':')?;
    let (l, c) = rest.split_once(':')?;
    let l: usize = l.parse().ok()?;
    let c: usize = c.parse().ok()?;
    if l >= 1 && c >= 1 {
        Some((l, c))
    } else {
        None
    }
}

#[test]
fn usage_on_help_no_args_and_unknown_args() {
    let argsets: Vec<Vec<&str>> = vec![vec![], vec!["--help"], vec!["--bogus"], vec!["help"]];
    for args in &argsets {
        let out = run(args, "");
        assert_eq!(
            out.code, Some(0),
            "args {args:?} should exit 0, got {:?} (stderr: {})",
            out.code,
            esc(&out.stderr)
        );
        assert!(
            out.stderr.is_empty(),
            "args {args:?} wrote to stderr: {}",
            esc(&out.stderr)
        );
        let low = out.stdout.to_lowercase();
        assert!(
            low.contains("usage"),
            "args {args:?} should print usage on stdout, got: {}",
            esc(&out.stdout)
        );
        assert!(
            low.contains("nf") && low.contains("type"),
            "args {args:?} usage text should mention `nf` and `type`, got: {}",
            esc(&out.stdout)
        );
    }
}

/// DEVIATION (NOTES.md #11): a leading UTF-8 BOM (`EF BB BF`, i.e. U+FEFF) is
/// treated as whitespace.  The bytes written here are exactly `\xEF\xBB\xBF` +
/// `U` + `\n`; the Haskell parser rejects the BOM.
#[test]
fn bom_is_accepted_deviation() {
    let out = run(&["nf"], "\u{feff}U\n");
    assert_eq!(
        out.code, Some(0),
        "BOM + U should exit 0, got {:?} (stderr: {})",
        out.code,
        esc(&out.stderr)
    );
    assert_eq!(out.stdout, "U\n  :\nU1\n", "BOM + U normal form/type");
    assert!(
        out.stderr.is_empty(),
        "unexpected stderr: {}",
        esc(&out.stderr)
    );
}

/// DEVIATION (NOTES.md #9): the echoed source line is windowed to 160
/// characters plus a trailing `\u{2026}`, and a name quoted in a message is
/// truncated after 64 characters plus `\u{2026}`, so a huge line cannot blow
/// the error report up.  Both effects are pinned below.
#[test]
fn long_line_errors_are_windowed_deviation() {
    // 100 000-character line with a short diagnostic: stderr stays tiny.
    let garbage = format!("@{}", "a".repeat(99_999));
    let out = run(&["nf"], &garbage);
    assert_eq!(
        out.code, Some(1),
        "100k-char garbage line should exit 1, got {:?}",
        out.code
    );
    assert!(out.stdout.is_empty(), "parse error wrote to stdout");
    assert!(
        out.stderr.len() < 1000,
        "stderr should stay bounded for a short diagnostic, got {} bytes:\n{}",
        out.stderr.len(),
        esc(&out.stderr)
    );
    let lines: Vec<&str> = out.stderr.lines().collect();
    assert_eq!(lines.first().copied(), Some("<stdin>:1:1:"), "header");
    assert_eq!(lines.get(1).copied(), Some("  |"), "padding line");
    assert_eq!(
        lines.get(2).map(|l| l.chars().count()),
        Some(165),
        "echoed line should be `1 | ` + 160 characters + `\u{2026}`, got {:?}",
        lines.get(2)
    );
    assert!(
        lines[2].starts_with("1 | ") && lines[2].ends_with('\u{2026}'),
        "echoed line is not windowed: {:?}",
        lines[2]
    );
    assert_eq!(
        lines.get(3).copied(),
        Some("  | ^"),
        "caret line (column 1)"
    );
    assert_eq!(lines.get(4).copied(), Some("expected an atom"), "message");

    // The requested variant: a 100 000-character *identifier*.  The echoed line
    // is windowed, and the name inside the message is truncated after 64
    // characters plus `\u{2026}`, so stderr stays tiny.
    let ident = "a".repeat(100_000);
    let out = run(&["nf"], &ident);
    assert_eq!(
        out.code, Some(1),
        "100k-char identifier should exit 1, got {:?}",
        out.code
    );
    assert!(out.stdout.is_empty(), "error wrote to stdout");
    let lines: Vec<&str> = out.stderr.lines().collect();
    assert_eq!(lines.first().copied(), Some("<stdin>:1:1:"), "header");
    assert_eq!(
        lines.get(2).map(|l| l.chars().count()),
        Some(165),
        "echoed line should be windowed even for the identifier input"
    );
    assert!(
        lines[2].starts_with("1 | ")
            && lines[2].ends_with('\u{2026}')
            && lines[2].chars().filter(|c| *c == 'a').count() == 160,
        "echoed line is not windowed: {:?}",
        lines[2]
    );
    assert!(
        out.stderr.len() < 1000,
        "the truncated message should keep stderr tiny, got {} bytes",
        out.stderr.len()
    );
    let prefix = "unbound variable: ";
    let want_msg = format!("{prefix}{}\u{2026}", "a".repeat(64));
    let msg = lines.last().copied().unwrap_or("<missing>");
    assert_eq!(msg, want_msg, "truncated unbound-variable message");
    assert!(
        msg.starts_with(prefix)
            && msg.ends_with('\u{2026}')
            && msg.chars().count() <= prefix.len() + 65,
        "message should be `unbound variable: ` + at most 64 characters + `\u{2026}`, got {:?}",
        msg
    );
}
