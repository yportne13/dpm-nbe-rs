# Re-verification of categories 2/3 and the keyword-position probes against the
# rebuilt binary (hash in the report).  ASCII only.
. "$PSScriptRoot\common.ps1"

# depth guards that exist (raw() based)
Invoke-Probe -Name v2_paren_511 -Category depth -Args @('nf') -Text (('(' * 511) + 'U' + (')' * 511)) -TimeoutSec 30
Invoke-Probe -Name v2_paren_512 -Category depth -Args @('nf') -Text (('(' * 512) + 'U' + (')' * 512)) -TimeoutSec 30
Invoke-Probe -Name v2_arrow_500 -Category depth -Args @('nf') -Text ((@('Nat') + (1..500 | ForEach-Object { '-> Nat' })) -join ' ') -TimeoutSec 30
Invoke-Probe -Name v2_arrow_600 -Category depth -Args @('nf') -Text ((@('Nat') + (1..600 | ForEach-Object { '-> Nat' })) -join ' ') -TimeoutSec 30
Invoke-Probe -Name v2_lets_500 -Category depth -Args @('nf') -Text (((1..500 | ForEach-Object { "let x$_ : Nat = zero; " }) -join '') + 'U') -TimeoutSec 30
Invoke-Probe -Name v2_lets_1000 -Category depth -Args @('nf') -Text (((1..1000 | ForEach-Object { "let x$_ : Nat = zero; " }) -join '') + 'U') -TimeoutSec 30
Invoke-Probe -Name v2_plusnest_500 -Category depth -Args @('nf') -Text (((1..500 | ForEach-Object { 'plus zero (' }) -join '') + 'zero' + (')' * 500)) -TimeoutSec 30
Invoke-Probe -Name v2_plusnest_600 -Category depth -Args @('nf') -Text (((1..600 | ForEach-Object { 'plus zero (' }) -join '') + 'zero' + (')' * 600)) -TimeoutSec 30

# unguarded recursions (suc chain, flat spine)
foreach ($n in 10000, 20000, 30000, 50000) {
    Invoke-Probe -Name "v2_suc_$n" -Category depth -Args @('nf') -Text (('suc ' * $n) + 'zero') -TimeoutSec 90 -Note "suc tower depth=$n"
}
foreach ($n in 20000, 50000) {
    Invoke-Probe -Name "v2_spineU_$n" -Category depth -Args @('nf') -Text ('U ' * $n) -TimeoutSec 90 -Note "flat U spine x$n"
}
Invoke-Probe -Name v2_spinezero_100000 -Category depth -Args @('nf') -Text ('zero ' * 100000) -TimeoutSec 90 -Note "flat zero spine x100000"

# transN scaling
function New-Trans2([int]$n) {
    $binders = (1..$n | ForEach-Object { "(a$_ : A) " }) -join ''
    $args = (1..($n - 1) | ForEach-Object { "-> Id A a$_ a$($_ + 1) " }) -join ''
    $vars = (1..$n | ForEach-Object { "a$_ " }) -join ''
    $refls = (1..($n - 1) | ForEach-Object { 'refl ' }) -join ''
    return "let t : (A : U) $binders$args-> Id A a1 a$n = \ A $vars$refls. refl; U"
}
foreach ($n in 10, 14, 16, 18, 20, 24) {
    Invoke-Probe -Name "v2_trans_$n" -Category exponential -Args @('nf') -Text (New-Trans2 $n) -TimeoutSec 300 -Note "transN n=$n"
}

# keyword-position probes (category 6)
Invoke-Probe -Name v2_kw_suc_let -Category position -Args @('nf') -Text "suc let" -Note "let at col 5"
Invoke-Probe -Name v2_kw_in -Category position -Args @('nf') -Text "U in" -Note "in at col 3"
Invoke-Probe -Name v2_kw_plus_let -Category position -Args @('nf') -Text "plus zero let" -Note "let at col 11"
Invoke-Probe -Name v2_kw_let_only -Category position -Args @('nf') -Text "let" -Note "bare let"
Invoke-Probe -Name v2_kw_suc_nat -Category position -Args @('nf') -Text "suc Nat" -Note "control: type error at col 5"

# absurd-lambda printing, now expected to succeed
Invoke-Probe -Name v2_print_absurd -Category panic -Args @('nf') -Text 'let C : (A : U) -> Id Nat (suc zero) zero -> A = \ A (!); C' -Note "print absurd lambda"
Invoke-Probe -Name v2_print_absurd_applied -Category panic -Args @('nf') -Text 'let C : (A : U) -> Id Nat (suc zero) zero -> A = \ A (!); C U' -Note "print applied absurd lambda"
Invoke-Probe -Name v2_absurd_index -Category panic -Args @('nf') -Text 'let C : (A : U) -> Id Nat (suc zero) zero -> A = \ A (!); Id ((A : U) -> Id Nat (suc zero) zero -> A) C C' -Note "absurd lambda as Id index"

# edge input re-check
Invoke-Probe -Name v2_bom -Category malformed -Args @('nf') -Bytes ([byte[]](0xEF,0xBB,0xBF,0x55,0x0A)) -Note "UTF-8 BOM"
Invoke-Probe -Name v2_nonutf8 -Category malformed -Args @('nf') -Bytes ([byte[]](0x55,0xFF,0xFE,0x0A)) -Note "invalid UTF-8"
Invoke-Probe -Name v2_unterm_comment -Category malformed -Args @('nf') -Text "U {- never closed" -Note "unterminated block comment"
Invoke-Probe -Name v2_empty -Category malformed -Args @('nf') -Text "" -Note "empty stdin"
Invoke-Probe -Name v2_tab_caret -Category position -Args @('nf') -Text "U`n`tlet z : Nat = U; z" -Note "tab caret alignment"
Invoke-Probe -Name v2_long_ident -Category malformed -Args @('nf') -Text ('a' * 100000) -Note "100k identifier stderr size"

Save-Results -Path (Join-Path $script:Root 'results_v2.json')
