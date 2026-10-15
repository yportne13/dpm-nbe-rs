//! Black-box golden tests for the `dpm-nbe` binary.
//!
//! Every expected value below was derived by hand from the Haskell reference
//! implementation in `../src/*.hs` and `../README.md`, *not* from the Rust port:
//!
//!   * `nf`   : `print (nf emptyEnv t); putStrLn "  :"; print (quote 0 a)`
//!   * `type` : `print (quote 0 a)`
//!   * terms are rendered by `Pretty.hs`, which
//!       - uses the Unicode `λ`, `→` and `⊥`,
//!       - prints a constructor as `succ` (while the parser accepts `suc`),
//!       - prints consecutive *named* Pi binders back to back with no separator
//!         (`(m : Nat)(n : Nat) → ...`, see `goPi`),
//!       - prints a lambda with the pattern names stored in the core term
//!         (`\ n . plus n zero` normalises to `λ n . plus n zero`).
//!
//! Error rendering (`Main.hs:displayError`) is
//!
//! ```text
//! <stdin>:L:C:
//! <pad> |
//! L | <source line>
//! <pad> | <caret>
//! <message>
//! ```
//!
//! where `<pad>` is as wide as the line number.  The whole rendering is pinned
//! byte-for-byte, including the caret column: the caret line is
//! `"{pad} | {col - 1 spaces}^"`, so the caret sits under the offending source
//! character.
//!
//! # Documented port deviations from the Haskell reference
//!
//! `../NOTES.md` (i.e. `rust/NOTES.md`) is the authoritative list: 13 numbered
//! deviations plus the inherited limitations.  This suite pins them as
//! follows.  Cases whose name ends in `_DEVIATION` (table entries) or
//! `_deviation` (the two standalone tests), plus the lambda-printing cases
//! (NOTES.md #1 and #3), encode the port's behaviour; everything else encodes
//! the Haskell behaviour.
//!
//! * The λ-printing expectations -- NOTES.md **#1** (`eval::frc_ne`: forcing a
//!   bare lambda terminates, so a lambda normal form is printable at all; the
//!   Haskell `Force Ne` re-enters `napp`/`frc` forever) and **#3** (`Quote Ne`
//!   rendering, `Lam ps (quote (l + n) (capp n cl (newVars l n)))`).
//! * `nf_partial_application_*_DEVIATION` -- NOTES.md **#3**: quotation
//!   saturates, so `k zero` for `k = λ x y . plus x y` prints `λ y . y` where
//!   Haskell's (unreachable) `Quote Ne` would drop the spine.
//! * `nf_C_absurd_value_DEVIATION` -- NOTES.md **#7** (`syntax::Val::Bot` has a
//!   value; `Eval.hs` has no `eval` case for `Bot` and panics with
//!   `error "eval: impossible"`).
//! * `'(!)'` in the "It should not be 'refl' ..." message -- NOTES.md **#8**
//!   (`Elab.hs` passes the literal `"!"`, which is not surface syntax).
//! * `nf_refl_plus_neutral_reflexive_DEVIATION` -- NOTES.md **#13**
//!   (`eval::conv_ne` compares stuck `plus` neutrals; Haskell's `convNe` has no
//!   `NPlus` case, so it rejects `refl : Id Nat (plus n m) (plus n m)`).
//! * Parse errors on **stderr** with exit code 1 -- NOTES.md **#9** (Haskell
//!   prints the megaparsec bundle to stdout and `exitSuccess`s).
//! * `bom_is_accepted_deviation` (a leading U+FEFF is whitespace) -- NOTES.md
//!   **#11** -- and `long_line_errors_are_windowed_deviation` (the echoed source
//!   line is windowed to 160 characters plus a trailing `…`, and a name quoted
//!   in a message is truncated after 64 characters plus `…`) -- NOTES.md **#9**.
//!   Both keep the `deviation` marker in their names because they encode port
//!   behaviour the original rejects or cannot emit.
//!
//! Only std is used; the binary is located through `CARGO_BIN_EXE_dpm-nbe`.

use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_dpm-nbe");

/// The whole program from README.md ("Example" block): trans, cong, H, C, U.
const README_FULL: &str = include_str!("fixtures/readme_full.txt");

// ---------------------------------------------------------------------------
// messages copied from Elab.hs
// ---------------------------------------------------------------------------

const M_LAMBDA: &str = "Can't infer type for lambda expression";
const M_REFL: &str = "Can't infer type for refl";
const M_MISMATCH: &str = "type mismatch\n\nexpected type:\n\n  Nat\n\ninferred type:\n\n  U";
const M_NOT_FUN: &str = "Expected a function type, instead inferred:\n\n  Nat";
const M_UNBOUND_X: &str = "unbound variable: x";
const M_UNBOUND_Z: &str = "unbound variable: z";
const M_EXPECT_ID_REFL: &str = "Expected an identity type for pattern 'refl', but got:\n\n  Nat";
const M_EXPECT_ID_ABS: &str = "Expected an identity type for pattern '(!)', but got:\n\n  Nat";
const M_TOO_MANY: &str = "Too many patterns.";
const M_NOT_SURE: &str = "I am not sure if there should be a pattern 'refl' for equation between plus m zero and succ zero.";
// DEVIATION 4: Elab.hs passes the literal "!", the port suggests the surface
// pattern `(!)`.
const M_SHOULD_NOT_REFL: &str =
    "It should not be 'refl' for equation between zero and succ zero, in fact you should try pattern '(!)'.";
const M_SHOULD_NOT_ABS: &str =
    "It should not be '(!)' for equation between zero and zero, in fact you should try pattern 'refl'.";

// ---------------------------------------------------------------------------
// case tables
// ---------------------------------------------------------------------------

/// (name, mode, source, expected stdout) for exit-code-0 runs.
static OK: &[(&str, &str, &str, &str)] = &[
    // --- README examples, each wrapped as `let <name> : <ty> = <body>; U` ---
    (
        "readme_sym",
        "nf",
        r"let sym : (A : U) (a b : A) -> Id A a b -> Id A b a = λ A a b refl . refl; U",
        "U\n  :\nU\n",
    ),
    (
        "readme_trans",
        "nf",
        r"let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl . refl; U",
        "U\n  :\nU\n",
    ),
    (
        "readme_cong",
        "nf",
        r"let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) = λ A B f a b refl . refl; U",
        "U\n  :\nU\n",
    ),
    (
        "readme_H",
        "nf",
        r"let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n refl refl . refl; U",
        "U\n  :\nU\n",
    ),
    (
        "readme_C_absurd",
        "nf",
        r"let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); U",
        "U\n  :\nU\n",
    ),
    ("readme_full_program", "nf", README_FULL, "U\n  :\nU\n"),
    // DEVIATION 3: the Haskell reference cannot print an absurd-pattern normal
    // form (it hangs in `frc`; forcing the closure would evaluate `Bot`).
    (
        "nf_C_absurd_value_DEVIATION",
        "nf",
        r"let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C",
        "λ A (!) . ⊥\n  :\n(A : U) → Id Nat (succ zero) zero → A\n",
    ),
    // --- normal forms of closed terms --------------------------------
    (
        "nf_plus_ss",
        "nf",
        r"plus (suc zero) (suc zero)",
        "succ (succ zero)\n  :\nNat\n",
    ),
    (
        "nf_plus_s2_z",
        "nf",
        r"plus (suc (suc zero)) zero",
        "succ (succ zero)\n  :\nNat\n",
    ),
    (
        "nf_plus_s_z",
        "nf",
        r"plus (suc zero) zero",
        "succ zero\n  :\nNat\n",
    ),
    ("nf_plus_zz", "nf", r"plus zero zero", "zero\n  :\nNat\n"),
    (
        "nf_let_plus_xx",
        "nf",
        r"let x : Nat = zero; plus x x",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_let_plus_x_sx",
        "nf",
        r"let x : Nat = suc zero; plus x x",
        "succ (succ zero)\n  :\nNat\n",
    ),
    ("nf_suc_zero", "nf", r"suc zero", "succ zero\n  :\nNat\n"),
    (
        "nf_suc_suc_zero",
        "nf",
        r"suc (suc zero)",
        "succ (succ zero)\n  :\nNat\n",
    ),
    ("nf_zero", "nf", r"zero", "zero\n  :\nNat\n"),
    ("nf_U", "nf", r"U", "U\n  :\nU\n"),
    (
        "nf_let_id_applied",
        "nf",
        r"let f : Nat -> Nat = λ x . x; f (suc zero)",
        "succ zero\n  :\nNat\n",
    ),
    (
        "nf_let_id_zero",
        "nf",
        r"let f : Nat -> Nat = λ x . x; f zero",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_backslash_lambda",
        "nf",
        r"let f : Nat -> Nat = \ x . x; f zero",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_arrow_is_a_type",
        "nf",
        r"Nat -> Nat",
        "Nat → Nat\n  :\nU\n",
    ),
    (
        "nf_unicode_arrow",
        "nf",
        r"let f : Nat → Nat = λ x . x; f (suc zero)",
        "succ zero\n  :\nNat\n",
    ),
    // The body of `Pi "_" a b` is printed in a scope whose binder is `_`, so an
    // occurrence of the *enclosing* variable resolves to that enclosing name.
    (
        "nf_anonymous_pi_body",
        "nf",
        r"(A : U) -> A -> A",
        "(A : U) → A → A\n  :\nU\n",
    ),
    // DEVIATION 1: the Haskell reference cannot reach any of the renderings
    // below (its `Force Ne` on a bare lambda re-enters `napp`/`frc` forever);
    // the port prints exactly what `Eval.hs`'s `Quote Ne` prescribes.
    (
        "nf_poly_identity_names",
        "nf",
        r"let f : (A : U) -> A -> A = λ A x . x; f",
        "λ A x . x\n  :\n(A : U) → A → A\n",
    ),
    (
        "nf_stuck_plus_right",
        "nf",
        r"let f : Nat -> Nat = λ n . plus n zero; f",
        "λ n . plus n zero\n  :\nNat → Nat\n",
    ),
    (
        "nf_plus_zero_left_is_id",
        "nf",
        r"let f : Nat -> Nat = λ n . plus zero n; f",
        "λ n . n\n  :\nNat → Nat\n",
    ),
    // DEVIATION 1: Haskell quotes a partially applied lambda by ignoring its
    // spine (`λ x y . plus x y` here); the port saturates and quotes the reduced
    // result, which is the actual normal form of `k zero`.
    (
        "nf_partial_application_of_lambda_DEVIATION",
        "nf",
        r"let k : Nat -> Nat -> Nat = λ x y . plus x y; k zero",
        "λ y . y\n  :\nNat → Nat\n",
    ),
    (
        "nf_partial_application_saturated_succ_DEVIATION",
        "nf",
        r"let k : Nat -> Nat -> Nat = λ x y . plus x y; k (suc zero)",
        "λ y . succ y\n  :\nNat → Nat\n",
    ),
    // DEVIATION 5: `convNe` has no `NPlus` case, so Haskell rejects `refl`
    // against `Id Nat (plus n m) (plus n m)`; the port compares stuck `plus`
    // neutrals and accepts it.
    (
        "nf_refl_plus_neutral_reflexive_DEVIATION",
        "nf",
        r"let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \ n m . refl; f",
        "λ n m . refl\n  :\n(n : Nat)(m : Nat) → Id Nat (plus n m) (plus n m)\n",
    ),
    (
        "nf_id_nat_zero_zero",
        "nf",
        r"Id Nat zero zero",
        "Id Nat zero zero\n  :\nU\n",
    ),
    (
        "nf_id_nat_suc_zero_zero",
        "nf",
        r"Id Nat (suc zero) zero",
        "Id Nat (succ zero) zero\n  :\nU\n",
    ),
    ("nf_id_U_nat_nat", "nf", r"Id U Nat Nat", "Id U Nat Nat\n  :\nU\n"),
    (
        "nf_let_type_alias",
        "nf",
        r"let A : U = Nat; A",
        "Nat\n  :\nU\n",
    ),
    (
        "nf_nested_let",
        "nf",
        r"let a : Nat = zero; let b : Nat = a; b",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_let_chain_plus",
        "nf",
        r"let x : Nat = suc zero; let y : Nat = x; plus y y",
        "succ (succ zero)\n  :\nNat\n",
    ),
    (
        "nf_shadowing_let",
        "nf",
        r"let x : Nat = zero; let x : Nat = suc x; x",
        "succ zero\n  :\nNat\n",
    ),
    (
        "nf_lam_two_args",
        "nf",
        r"let g : Nat -> Nat -> Nat = λ x y . plus x y; g",
        "λ x y . plus x y\n  :\nNat → Nat → Nat\n",
    ),
    (
        "nf_named_dependent_pi",
        "nf",
        r"(n : Nat) -> Id Nat n n",
        "(n : Nat) → Id Nat n n\n  :\nU\n",
    ),
    (
        "nf_two_named_binders",
        "nf",
        r"(m : Nat) (n : Nat) -> Nat",
        "(m : Nat)(n : Nat) → Nat\n  :\nU\n",
    ),
    (
        "nf_let_refl_value",
        "nf",
        r"let p : Id Nat zero zero = refl; p",
        "refl\n  :\nId Nat zero zero\n",
    ),
    (
        "nf_let_poly_const_applied",
        "nf",
        r"let f : (A : U) -> A -> Nat = λ A x . zero; f Nat zero",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_plus_nested",
        "nf",
        r"plus (plus (suc zero) zero) (suc zero)",
        "succ (succ zero)\n  :\nNat\n",
    ),
    (
        "nf_dependent_function",
        "nf",
        r"let f : (m : Nat) -> Id Nat m (plus m zero) -> Nat = λ m p . zero; f",
        "λ m p . zero\n  :\n(m : Nat) → Id Nat m (plus m zero) → Nat\n",
    ),
    (
        "nf_H_value",
        "nf",
        r"let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n refl refl . refl; H",
        "λ m n refl refl . refl\n  :\n(m : Nat)(n : Nat) → Id Nat n (plus m zero) → Id Nat m zero → Id Nat n zero\n",
    ),
    (
        "nf_trans_value",
        "nf",
        r"let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl . refl; trans",
        "λ A a b c refl refl . refl\n  :\n(A : U)(a : A)(b : A)(c : A) → Id A a b → Id A b c → Id A a c\n",
    ),
    (
        "nf_sym_value",
        "nf",
        r"let sym : (A : U) (a b : A) -> Id A a b -> Id A b a = λ A a b refl . refl; sym",
        "λ A a b refl . refl\n  :\n(A : U)(a : A)(b : A) → Id A a b → Id A b a\n",
    ),
    (
        "nf_cong_value",
        "nf",
        r"let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) = λ A B f a b refl . refl; cong",
        "λ A B f a b refl . refl\n  :\n(A : U)(B : U)(f : A → B)(a : A)(b : A) → Id A a b → Id B (f a) (f b)\n",
    ),
    // --- comments ----------------------------------------------------
    (
        "nf_line_comment",
        "nf",
        "let x : Nat = zero; -- comment\nplus x x",
        "zero\n  :\nNat\n",
    ),
    (
        "nf_block_comment",
        "nf",
        r"let x : Nat = {- hi -} suc zero; plus x zero",
        "succ zero\n  :\nNat\n",
    ),
    // --- `type` mode -------------------------------------------------
    ("type_Nat", "type", r"Nat", "U\n"),
    ("type_zero", "type", r"zero", "Nat\n"),
    ("type_suc_zero", "type", r"suc zero", "Nat\n"),
    ("type_suc_suc_zero", "type", r"suc (suc zero)", "Nat\n"),
    ("type_plus_zz", "type", r"plus zero zero", "Nat\n"),
    ("type_id_nat_zz", "type", r"Id Nat zero zero", "U\n"),
    ("type_id_U_nat_nat", "type", r"Id U Nat Nat", "U\n"),
    ("type_pi_anonymous", "type", r"(A : U) -> A -> A", "U\n"),
    ("type_U", "type", r"U", "U\n"),
    ("type_arrow", "type", r"Nat -> Nat", "U\n"),
    ("type_unicode_arrow", "type", r"Nat → Nat", "U\n"),
    ("type_named_pi", "type", r"(n : Nat) -> Id Nat n n", "U\n"),
    (
        "type_let_id",
        "type",
        r"let f : Nat -> Nat = λ x . x; f",
        "Nat → Nat\n",
    ),
    (
        "type_let_dependent",
        "type",
        r"let f : (m : Nat) -> Id Nat m (plus m zero) -> Nat = λ m p . zero; f",
        "(m : Nat) → Id Nat m (plus m zero) → Nat\n",
    ),
    (
        "type_let_applied",
        "type",
        r"let f : Nat -> Nat = λ x . x; f zero",
        "Nat\n",
    ),
    ("type_let_var", "type", r"let x : Nat = zero; x", "Nat\n"),
    (
        "type_let_nested",
        "type",
        r"let a : Nat = zero; let b : Nat = a; b",
        "Nat\n",
    ),
];

/// (name, mode, source, line, col, message) for exit-code-1 elaboration errors.
static ERR: &[(&str, &str, &str, usize, usize, &str)] = &[
    ("err_lambda_nf", "nf", r"λ x . x", 1, 1, M_LAMBDA),
    ("err_backslash_lambda_nf", "nf", r"\ x . x", 1, 1, M_LAMBDA),
    ("err_lambda_type", "type", r"λ x . x", 1, 1, M_LAMBDA),
    ("err_refl_nf", "nf", r"refl", 1, 1, M_REFL),
    ("err_refl_type", "type", r"refl", 1, 1, M_REFL),
    ("err_unbound_x", "nf", r"x", 1, 1, M_UNBOUND_X),
    (
        "err_unbound_z_in_let",
        "nf",
        r"let y : Nat = z; y",
        1,
        15,
        M_UNBOUND_Z,
    ),
    (
        "err_mismatch_U_against_Nat",
        "nf",
        r"let x : Nat = U; x",
        1,
        15,
        M_MISMATCH,
    ),
    ("err_mismatch_suc_arg", "nf", r"suc U", 1, 5, M_MISMATCH),
    (
        "err_mismatch_type_mode",
        "type",
        r"let x : Nat = U; x",
        1,
        15,
        M_MISMATCH,
    ),
    ("err_not_a_function", "nf", r"zero zero", 1, 1, M_NOT_FUN),
    (
        "err_not_a_function_type_mode",
        "type",
        r"zero zero",
        1,
        1,
        M_NOT_FUN,
    ),
    (
        "err_refl_pattern_on_nat",
        "nf",
        r"let f : Nat -> Nat = λ refl . zero; U",
        1,
        22,
        M_EXPECT_ID_REFL,
    ),
    (
        "err_abs_pattern_on_nat",
        "nf",
        r"let f : Nat -> Nat -> Nat = λ x (!); U",
        1,
        29,
        M_EXPECT_ID_ABS,
    ),
    (
        "err_too_many_patterns",
        "nf",
        r"let f : Nat = λ x . zero; U",
        1,
        15,
        M_TOO_MANY,
    ),
    (
        "err_refl_not_sure",
        "nf",
        r"let f : (m : Nat) -> Id Nat (plus m zero) (suc zero) -> Nat = λ m refl . zero; U",
        1,
        63,
        M_NOT_SURE,
    ),
    (
        "err_refl_should_be_absurd",
        "nf",
        r"let f : Id Nat zero (suc zero) -> Nat = λ refl . zero; U",
        1,
        41,
        M_SHOULD_NOT_REFL,
    ),
    (
        "err_absurd_should_be_refl",
        "nf",
        r"let f : (x : Nat) -> Id Nat zero zero -> Nat = λ x (!); U",
        1,
        48,
        M_SHOULD_NOT_ABS,
    ),
    // position tracking on later lines
    (
        "err_second_line_not_a_function",
        "nf",
        "let x : Nat = zero;\nzero zero",
        2,
        1,
        M_NOT_FUN,
    ),
    (
        "err_second_line_mismatch",
        "nf",
        "let x : Nat = zero;\nsuc U",
        2,
        5,
        M_MISMATCH,
    ),
];

/// (name, source) garbage that must not parse.
static PARSE_ERR: &[(&str, &str)] = &[
    ("parse_plus_partial", r"plus zero"),
    ("parse_suc_partial", r"suc"),
    ("parse_id_partial", r"Id Nat zero"),
    ("parse_let_missing_colon", r"let x = zero; x"),
    ("parse_lam_no_pattern", r"λ . refl"),
    ("parse_garbage", r"@"),
    ("parse_unbalanced_paren", r"(A : U -> A"),
    ("parse_stray_rparen", r"Id Nat zero zero)"),
    ("parse_missing_body", r"let x : Nat = zero;"),
    ("parse_empty_input", r""),
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
    s.push_str("λ A ");
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
    let mut s = String::from("λ A");
    for i in 1..=n {
        s.push_str(&format!(" a{i}"));
    }
    for _ in 1..n {
        s.push_str(" refl");
    }
    s.push_str(" . refl");
    s
}

/// Expected type of `genTransN n`, as printed by Pretty.hs (`goPi` emits
/// consecutive named binders without a separator).
fn trans_ty(n: usize) -> String {
    let mut s = String::from("(A : U)");
    for i in 1..=n {
        s.push_str(&format!("(a{i} : A)"));
    }
    for i in 1..n {
        s.push_str(&format!(" → Id A a{i} a{}", i + 1));
    }
    s.push_str(&format!(" → Id A a1 a{n}"));
    s
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[test]
fn golden_success_cases() {
    let mut cases: Vec<(String, String, String, String)> = OK
        .iter()
        .map(|&(n, m, s, e)| (n.to_string(), m.to_string(), s.to_string(), e.to_string()))
        .collect();

    // `genTransN` family from Main.hs, n = 1..=6.  As written in Main.hs the
    // program ends in `U`, so the whole program normalises to `U : U`.
    for n in 1..=6usize {
        cases.push((
            format!("gen_trans_{n}"),
            "nf".to_string(),
            trans_prog(n, "U"),
            "U\n  :\nU\n".to_string(),
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
            "let trans1 : (A : U) (a1 : A) -> Id A a1 a1 = λ A a1 . refl; trans1",
            "λ A a1 . refl\n  :\n(A : U)(a1 : A) → Id A a1 a1\n",
        ),
        (
            "trans3_value",
            "let trans3 : (A : U) (a1 : A) (a2 : A) (a3 : A) -> Id A a1 a2 -> Id A a2 a3 -> Id A a1 a3 = λ A a1 a2 a3 refl refl . refl; trans3",
            "λ A a1 a2 a3 refl refl . refl\n  :\n(A : U)(a1 : A)(a2 : A)(a3 : A) → Id A a1 a2 → Id A a2 a3 → Id A a1 a3\n",
        ),
        (
            "trans6_value",
            "let trans6 : (A : U) (a1 : A) (a2 : A) (a3 : A) (a4 : A) (a5 : A) (a6 : A) -> Id A a1 a2 -> Id A a2 a3 -> Id A a3 a4 -> Id A a4 a5 -> Id A a5 a6 -> Id A a1 a6 = λ A a1 a2 a3 a4 a5 a6 refl refl refl refl refl . refl; trans6",
            "λ A a1 a2 a3 a4 a5 a6 refl refl refl refl refl . refl\n  :\n(A : U)(a1 : A)(a2 : A)(a3 : A)(a4 : A)(a5 : A)(a6 : A) → Id A a1 a2 → Id A a2 a3 → Id A a3 a4 → Id A a4 a5 → Id A a5 a6 → Id A a1 a6\n",
        ),
        (
            "trans1_program",
            "let trans1 : (A : U) (a1 : A) -> Id A a1 a1 = λ A a1 . refl; U",
            "U\n  :\nU\n",
        ),
        (
            "trans6_program",
            "let trans6 : (A : U) (a1 : A) (a2 : A) (a3 : A) (a4 : A) (a5 : A) (a6 : A) -> Id A a1 a2 -> Id A a2 a3 -> Id A a3 a4 -> Id A a4 a5 -> Id A a5 a6 -> Id A a1 a6 = λ A a1 a2 a3 a4 a5 a6 refl refl refl refl refl . refl; U",
            "U\n  :\nU\n",
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
        ERR.len() >= 20,
        "the error table must keep all cases, found {}",
        ERR.len()
    );
    let mut failures: Vec<String> = Vec::new();

    for &(name, mode, src, line, col, msg) in ERR {
        let out = run(&[mode], src);
        if out.code != Some(1) {
            failures.push(format!(
                "[{name}] exit code {:?}, expected 1\n    src: {}\n    stdout: {}\n    stderr:\n{}",
                out.code,
                esc(src),
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
                esc(src),
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
        // `displayError` pins the whole rendering: the padding line is
        // `{pad} |` and the caret line is `{pad} | {col - 1 spaces}^`, where
        // `pad` is as wide as the line number.
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

    for &(name, src) in PARSE_ERR {
        let out = run(&["nf"], src);
        if out.code != Some(1) {
            failures.push(format!(
                "[{name}] exit code {:?}, expected 1\n    src: {}\n    stdout: {}\n    stderr:\n{}",
                out.code,
                esc(src),
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
            out.code,
            Some(0),
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
        out.code,
        Some(0),
        "BOM + U should exit 0, got {:?} (stderr: {})",
        out.code,
        esc(&out.stderr)
    );
    assert_eq!(out.stdout, "U\n  :\nU\n", "BOM + U normal form/type");
    assert!(
        out.stderr.is_empty(),
        "unexpected stderr: {}",
        esc(&out.stderr)
    );
}

/// DEVIATION (NOTES.md #9): the echoed source line is windowed to 160
/// characters plus a trailing `…`, and a name quoted in a message is truncated
/// after 64 characters plus `…`, so a huge line cannot blow the error report up.
/// Both effects are pinned below.
#[test]
fn long_line_errors_are_windowed_deviation() {
    // 100 000-character line with a short diagnostic: stderr stays tiny.
    let garbage = format!("@{}", "a".repeat(99_999));
    let out = run(&["nf"], &garbage);
    assert_eq!(
        out.code,
        Some(1),
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
        "echoed line should be `1 | ` + 160 characters + `…`, got {:?}",
        lines.get(2)
    );
    assert!(
        lines[2].starts_with("1 | ") && lines[2].ends_with('…'),
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
    // characters plus `…`, so stderr stays tiny.
    let ident = "a".repeat(100_000);
    let out = run(&["nf"], &ident);
    assert_eq!(
        out.code,
        Some(1),
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
            && lines[2].ends_with('…')
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
    let want_msg = format!("{prefix}{}…", "a".repeat(64));
    let msg = lines.last().copied().unwrap_or("<missing>");
    assert_eq!(msg, want_msg, "truncated unbound-variable message");
    assert!(
        msg.starts_with(prefix) && msg.ends_with('…') && msg.chars().count() <= prefix.len() + 65,
        "message should be `unbound variable: ` + at most 64 characters + `…`, got {:?}",
        msg
    );
}
