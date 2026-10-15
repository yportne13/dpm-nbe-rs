# Category 1: panic hunting.
# Every candidate targets a known `panic!`/`unreachable!`/`assert!` site in src/.
. "$PSScriptRoot\common.ps1"

# ---- A. absurd-pattern lambdas (`Tm::Bot` reachable in eval/quote/conv) ----
$absT = "(A : U) -> Id Nat (suc zero) zero -> A"
Invoke-Probe -Name p_a1_print_absurd -Category panic -Args @('nf') `
    -Text "let C : $absT = \ A (!); C" -Note "nf must print an absurd lambda (quote -> capp -> eval Bot)"
Invoke-Probe -Name p_a1_print_absurd_type -Category panic -Args @('type') `
    -Text "let C : $absT = \ A (!); C" -Note "type mode of the same program"
Invoke-Probe -Name p_a2_print_absurd_alias -Category panic -Args @('nf') `
    -Text "let P : U = $absT; let C : P = \ A (!); C" -Note "absurd lambda under a type alias"
Invoke-Probe -Name p_a3_print_absurd_applied -Category panic -Args @('nf') `
    -Text "let C : $absT = \ A (!); C U" -Note "absurd lambda value carrying a spine"
Invoke-Probe -Name p_a4_absurd_as_index -Category panic -Args @('nf') `
    -Text "let C : $absT = \ A (!); Id ($absT) C C" -Note "absurd lambda as an Id index (quoted through Id)"
Invoke-Probe -Name p_a5_absurd_in_pi -Category panic -Args @('nf') `
    -Text "let C : $absT = \ A (!); (f : $absT) -> Nat" -Note "control: Pi mentioning the type only"
Invoke-Probe -Name p_a6_absurd_bound_only -Category panic -Args @('nf') `
    -Text "let C : $absT = \ A (!); U" -Note "control: bound but never printed (README case)"
Invoke-Probe -Name p_a7_absurd_conv_inline -Category panic -Args @('nf') `
    -Text "let T : U = $absT;`nlet h : Id T (\ A (!)) (\ A (!)) = refl;`nU" -Note "conv of two absurd lambdas"
Invoke-Probe -Name p_a8_absurd_conv_vars -Category panic -Args @('nf') `
    -Text "let T : U = $absT;`nlet f : T = \ A (!);`nlet g : T = \ A (!);`nlet h : Id T f g = refl;`nU" -Note "conv via bound absurd lambdas"
Invoke-Probe -Name p_a9_absurd_unify -Category panic -Args @('nf') `
    -Text "let H : Id ($absT) (\ A (!)) (\ A (!)) -> Nat = \ refl . zero;`nU" -Note "unify falls back to conv on absurd lambdas"
Invoke-Probe -Name p_a10_mixed_lam_conv -Category panic -Args @('nf') `
    -Text "let T : U = $absT;`nlet h : Id T (\ A (!)) (\ A a . a) = refl;`nU" -Note "absurd lambda vs ordinary lambda"

# ---- B. refl / unify / substitution bookkeeping (lookup_env, lvl2ix, level, subst) ----
Invoke-Probe -Name p_b1_sym -Category panic -Args @('nf') `
    -Text "let sym : (A : U) (a b : A) -> Id A a b -> Id A b a = \ A a b refl . refl; sym"
Invoke-Probe -Name p_b2_trans -Category panic -Args @('nf') `
    -Text "let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = \ A a b c refl refl . refl; trans"
Invoke-Probe -Name p_b3_cong -Category panic -Args @('nf') `
    -Text "let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) = \ A B f a b refl . refl; cong"
Invoke-Probe -Name p_b4_H -Category panic -Args @('nf') `
    -Text "let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = \ m n refl refl . refl; H"
Invoke-Probe -Name p_b5_three_refl -Category panic -Args @('nf') `
    -Text "let f : (a b c : Nat) -> Id Nat a b -> Id Nat b c -> Id Nat a c -> Nat = \ a b c refl refl refl . zero; f"
Invoke-Probe -Name p_b6_four_refl -Category panic -Args @('nf') `
    -Text "let f : (a b c d : Nat) -> Id Nat a b -> Id Nat b c -> Id Nat c d -> Id Nat a d -> Nat = \ a b c d refl refl refl refl . zero; f"
Invoke-Probe -Name p_b7_id_of_id -Category panic -Args @('nf') `
    -Text "let K : (A : U) (a : A) -> Id (Id A a a) refl refl -> Nat = \ A a refl . zero; K"
Invoke-Probe -Name p_b8_suc_inj -Category panic -Args @('nf') `
    -Text "let f : (n : Nat) -> Id Nat (suc n) (suc n) -> Nat = \ n refl . n; f"
Invoke-Probe -Name p_b9_plus_inj -Category panic -Args @('nf') `
    -Text "let f : (m n : Nat) -> Id Nat (plus m zero) n -> Id Nat m n -> Nat = \ m n refl refl . zero; f"
Invoke-Probe -Name p_b10_refine_then_apply -Category panic -Args @('nf') `
    -Text "let go : (A : U) (a : A) -> Nat = \ A a . zero;`nlet use : (A : U) (a : A) -> Id A a a -> Nat = \ A a refl . go A a; use"
Invoke-Probe -Name p_b11_absurd_and_refl -Category panic -Args @('nf') `
    -Text "let f : (A : U) -> Id Nat (suc zero) zero -> Id Nat zero zero -> A = \ A (!); f"
Invoke-Probe -Name p_b12_refl_two_levels -Category panic -Args @('nf') `
    -Text "let f : (A : U) (B : A -> U) (a b : A) -> Id A a b -> Id (B a) (B b) -> Nat = \ A B a b refl refl . zero; f"
Invoke-Probe -Name p_b13_refl_three_levels -Category panic -Args @('nf') `
    -Text "let f : (A : U) (B : A -> U) (a b : A) -> Id A a b -> Id (B a) (B b) -> Id (B b) (B a) -> Nat = \ A B a b refl refl refl . zero; f"
Invoke-Probe -Name p_b14_trans_then_plus -Category panic -Args @('nf') `
    -Text "let f : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero -> Nat = \ m n refl refl refl . zero; f"

# transN family (same generator as the golden tests).  NOTE: ASCII-only source,
# because Windows PowerShell 5.1 reads .ps1 files as ANSI.
function New-Trans([int]$n, [string]$final = 'U') {
    $binders = (1..$n | ForEach-Object { "(a$_ : A) " }) -join ''
    $args = (1..($n - 1) | ForEach-Object { "-> Id A a$_ a$($_ + 1) " }) -join ''
    $ret = "-> Id A a1 a$n"
    $vars = (1..$n | ForEach-Object { "a$_ " }) -join ''
    $refls = (1..($n - 1) | ForEach-Object { 'refl ' }) -join ''
    return "let trans$n : (A : U) $binders$args$ret = \ A $vars$refls. refl; $final"
}
foreach ($n in 2, 4, 6, 8, 10) {
    Invoke-Probe -Name "p_bT$n" -Category panic -Args @('nf') -Text (New-Trans $n) -Note "transN n=$n, body U"
}
foreach ($n in 3, 6) {
    Invoke-Probe -Name "p_bTt$n" -Category panic -Args @('type') -Text (New-Trans $n) -Note "transN n=$n in type mode"
}
foreach ($n in 2, 4) {
    Invoke-Probe -Name "p_bTp$n" -Category panic -Args @('nf') -Text (New-Trans $n "trans$n") -Note "transN n=$n printed as a lambda value"
}

# ---- C. clean-error controls (must NOT panic) ----
Invoke-Probe -Name p_c1_zero_zero -Category panic -Args @('nf') -Text "zero zero"                 -Note "apply a non-function"
Invoke-Probe -Name p_c2_U_U -Category panic -Args @('nf') -Text "U U"                            -Note "apply U"
Invoke-Probe -Name p_c3_Nat_Nat -Category panic -Args @('nf') -Text "Nat Nat"                    -Note "apply Nat"
Invoke-Probe -Name p_c4_refl_refl -Category panic -Args @('nf') -Text "refl refl"                -Note "apply refl"
Invoke-Probe -Name p_c5_lam_apply -Category panic -Args @('nf') -Text "(\ x . x) U"              -Note "apply an uninferable lambda"
Invoke-Probe -Name p_c6_plus_one_arg -Category panic -Args @('nf') -Text "plus zero"             -Note "plus under-applied"
Invoke-Probe -Name p_c7_id_two_args -Category panic -Args @('nf') -Text "Id Nat zero"            -Note "Id under-applied"
Invoke-Probe -Name p_c8_suc_bare -Category panic -Args @('nf') -Text "suc"                       -Note "suc with no argument"
Invoke-Probe -Name p_c9_absurd_non_id -Category panic -Args @('nf') `
    -Text "let f : (A : U) -> A -> A = \ A (!); U" -Note "(!) where the domain is not an Id type"
Invoke-Probe -Name p_c10_refl_non_id -Category panic -Args @('nf') `
    -Text "let f : Nat -> Nat = \ x refl . x; U" -Note "refl where the domain is not an Id type"
Invoke-Probe -Name p_c11_too_many_pats -Category panic -Args @('nf') `
    -Text "let f : Nat -> Nat = \ x y . x; U" -Note "too many patterns"
Invoke-Probe -Name p_c12_absurd_rhs -Category panic -Args @('nf') `
    -Text "let f : (A : U) -> Id Nat (suc zero) zero -> A = \ A (!) . U; U" -Note "(!) with a right-hand side"
Invoke-Probe -Name p_c13_refl_wrong_indices -Category panic -Args @('nf') `
    -Text "let f : Id Nat (suc zero) zero = refl; U" -Note "refl on a false equation"
Invoke-Probe -Name p_c14_lam_check_nat -Category panic -Args @('nf') `
    -Text "let f : Nat = (\ x . x) zero; f" -Note "lambda checked against Nat"
Invoke-Probe -Name p_c15_unify_mismatch -Category panic -Args @('nf') `
    -Text "let f : (A : U) (a b : A) -> Id A a b -> Id A b a -> Nat = \ A a b refl refl . zero; f"
Invoke-Probe -Name p_c16_absurd_idk -Category panic -Args @('nf') `
    -Text "let f : (A : U) (a : A) -> Id A a a -> Nat = \ A a (!); U" -Note "(!) on a true equation"
Invoke-Probe -Name p_c17_refl_on_var_eq -Category panic -Args @('nf') `
    -Text "let f : (A : U) (a : A) -> Id A a a -> Nat = \ A a refl . zero; U"

Save-Results -Path (Join-Path $script:Root 'results_panics.json')
