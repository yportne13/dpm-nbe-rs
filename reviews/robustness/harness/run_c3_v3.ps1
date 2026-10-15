# Verification against the newest revision (parser budgets for atom()/spine(),
# MAX_DEPTH = 2048).  ASCII only.
. "$PSScriptRoot\common.ps1"

# previously-aborting unguarded recursions
foreach ($n in 25000, 50000, 100000) {
    Invoke-Probe -Name "v3_suc_$n" -Category depth -Args @('nf') -Text (('suc ' * $n) + 'zero') -TimeoutSec 60 -Note "suc tower depth=$n"
}
foreach ($n in 50000, 100000) {
    Invoke-Probe -Name "v3_spineU_$n" -Category depth -Args @('nf') -Text ('U ' * $n) -TimeoutSec 60 -Note "flat U spine x$n"
}
Invoke-Probe -Name v3_spinezero_100000 -Category depth -Args @('nf') -Text ('zero ' * 100000) -TimeoutSec 60 -Note "flat zero spine x100000"
Invoke-Probe -Name v3_spineU_2048 -Category depth -Args @('nf') -Text ('U ' * 2048) -TimeoutSec 60 -Note "spine at the budget"
Invoke-Probe -Name v3_spineU_2049 -Category depth -Args @('nf') -Text ('U ' * 2049) -TimeoutSec 60 -Note "spine over the budget"

# new MAX_DEPTH boundary
Invoke-Probe -Name v3_paren_2047 -Category depth -Args @('nf') -Text (('(' * 2047) + 'U' + (')' * 2047)) -TimeoutSec 60
Invoke-Probe -Name v3_paren_2048 -Category depth -Args @('nf') -Text (('(' * 2048) + 'U' + (')' * 2048)) -TimeoutSec 60
Invoke-Probe -Name v3_paren_2049 -Category depth -Args @('nf') -Text (('(' * 2049) + 'U' + (')' * 2049)) -TimeoutSec 60
Invoke-Probe -Name v3_arrow_2047 -Category depth -Args @('nf') -Text ((@('Nat') + (1..2047 | ForEach-Object { '-> Nat' })) -join ' ') -TimeoutSec 60
Invoke-Probe -Name v3_arrow_2048 -Category depth -Args @('nf') -Text ((@('Nat') + (1..2048 | ForEach-Object { '-> Nat' })) -join ' ') -TimeoutSec 60
Invoke-Probe -Name v3_lets_2000 -Category depth -Args @('nf') -Text (((1..2000 | ForEach-Object { "let x$_ : Nat = zero; " }) -join '') + 'U') -TimeoutSec 60
Invoke-Probe -Name v3_lets_3000 -Category depth -Args @('nf') -Text (((1..3000 | ForEach-Object { "let x$_ : Nat = zero; " }) -join '') + 'U') -TimeoutSec 60
Invoke-Probe -Name v3_suc_2048 -Category depth -Args @('nf') -Text (('suc ' * 2048) + 'zero') -TimeoutSec 60 -Note "suc at the budget"
Invoke-Probe -Name v3_suc_2049 -Category depth -Args @('nf') -Text (('suc ' * 2049) + 'zero') -TimeoutSec 60 -Note "suc over the budget"

# resource scaling still fine?
function New-T4([int]$n) {
    $b = (1..$n | ForEach-Object { "(a$_ : A) " }) -join ''
    $ar = (1..($n - 1) | ForEach-Object { "-> Id A a$_ a$($_ + 1) " }) -join ''
    $v = (1..$n | ForEach-Object { "a$_ " }) -join ''
    $rf = (1..($n - 1) | ForEach-Object { 'refl ' }) -join ''
    return "let t : (A : U) $b$ar-> Id A a1 a$n = \ A $v$rf. refl; U"
}
foreach ($n in 30, 100, 200) {
    Invoke-Probe -Name "v3_trans_$n" -Category exponential -Args @('nf') -Text (New-T4 $n) -TimeoutSec 120 -Note "transN n=$n"
}
$tower = ('suc ' * 1000) + 'zero'
Invoke-Probe -Name v3_plus_1000_1000 -Category exponential -Args @('nf') -Text ("plus ($tower) ($tower)") -TimeoutSec 60 -Note "plus of two 1000 towers"

# remaining edge/position findings
Invoke-Probe -Name v3_bom -Category malformed -Args @('nf') -Bytes ([byte[]](0xEF,0xBB,0xBF,0x55,0x0A)) -Note "UTF-8 BOM"
Invoke-Probe -Name v3_nonutf8 -Category malformed -Args @('nf') -Bytes ([byte[]](0x55,0xFF,0xFE,0x0A)) -Note "invalid UTF-8"
Invoke-Probe -Name v3_unterm -Category malformed -Args @('nf') -Text "U {- never closed" -Note "unterminated comment"
Invoke-Probe -Name v3_kw_suc_let -Category position -Args @('nf') -Text "suc let" -Note "let at col 5"
Invoke-Probe -Name v3_kw_in -Category position -Args @('nf') -Text "U in" -Note "in at col 3"
Invoke-Probe -Name v3_tab_caret -Category position -Args @('nf') -Text "U`n`tlet z : Nat = U; z" -Note "tab caret"
Invoke-Probe -Name v3_long_ident -Category malformed -Args @('nf') -Text ('a' * 100000) -Note "100k identifier"
Invoke-Probe -Name v3_print_absurd -Category panic -Args @('nf') -Text 'let C : (A : U) -> Id Nat (suc zero) zero -> A = \ A (!); C' -Note "print absurd lambda"

Save-Results -Path (Join-Path $script:Root 'results_v3.json')
