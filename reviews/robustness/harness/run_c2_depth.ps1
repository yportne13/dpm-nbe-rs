# Category 2: depth limits, stack safety and hangs.
# NOTE: ASCII only, because Windows PowerShell 5.1 reads .ps1 files as ANSI.
. "$PSScriptRoot\common.ps1"

# ---- parenthesised nesting (guarded by parse::MAX_DEPTH = 512) ----
foreach ($n in 500, 511, 512, 513, 600, 2000) {
    $src = ('(' * $n) + 'U' + (')' * $n)
    Invoke-Probe -Name "d_paren_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 30 `
        -Note "nested parens depth=$n"
}

# ---- arrow chains (each codomain goes through raw(), so guarded) ----
foreach ($n in 500, 600, 5000) {
    $src = (@('Nat') + (1..$n | ForEach-Object { '-> Nat' })) -join ' '
    Invoke-Probe -Name "d_arrow_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 30 `
        -Note "arrow chain length=$n"
}

# ---- lambda chains (body goes through raw()) ----
foreach ($n in 500, 600, 2000) {
    $src = ('\ x . ' * $n) + 'x'
    Invoke-Probe -Name "d_lam_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 30 `
        -Note "lambda chain length=$n"
}

# ---- suc chains: atom() recurses directly, NOT counted by raw()/MAX_DEPTH ----
foreach ($n in 400, 512, 1000, 10000, 50000, 100000, 300000, 1000000) {
    $src = ('suc ' * $n) + 'zero'
    Invoke-Probe -Name "d_suc_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 60 `
        -Note "suc tower depth=$n"
}

# ---- flat application spines: spine() is a loop, but infer/check recurse per node ----
foreach ($n in 10000, 100000, 300000, 1000000, 3000000) {
    $src = ('U ' * $n)
    Invoke-Probe -Name "d_spine_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 60 `
        -Note "flat spine of U x$n"
}
foreach ($n in 100000, 500000) {
    $src = ('zero ' * $n)
    Invoke-Probe -Name "d_spine_zero_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 60 `
        -Note "flat spine of zero x$n"
}

# ---- nested lets (each nesting level passes through raw()) ----
foreach ($n in 100, 400, 500, 1000) {
    $src = ((1..$n | ForEach-Object { "let x$_ : Nat = zero; " }) -join '') + 'U'
    Invoke-Probe -Name "d_lets_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 30 `
        -Note "nested lets n=$n"
}

# ---- plus towers and big numerals (reduction builds deep values) ----
$sucTower = { param($k) ('suc ' * $k) + 'zero' }
foreach ($k in 200, 400) {
    $src = 'plus (' + (& $sucTower $k) + ') ' + (& $sucTower $k)
    Invoke-Probe -Name "d_plus_$k" -Category depth -Args @('nf') -Text $src -TimeoutSec 60 `
        -Note "plus of two $k-deep towers (result 2*$k)"
}
# nesting plus through parens is depth-guarded; find the boundary
foreach ($n in 400, 500, 600) {
    $src = (1..$n | ForEach-Object { 'plus zero (' }) -join ''
    $src = $src + 'zero' + (')' * $n)
    Invoke-Probe -Name "d_plusnest_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 30 `
        -Note "nested plus depth=$n"
}

# ---- Id nesting ----
foreach ($n in 100, 200) {
    $src = ('Id Nat (' * $n) + 'zero' + (')' * $n) + ' zero'
    Invoke-Probe -Name "d_idnest_$n" -Category depth -Args @('nf') -Text $src -TimeoutSec 30 `
        -Note "nested Id depth=$n"
}

# ---- transN scaling (quadratic/cubic behaviour?) ----
function New-Trans([int]$n) {
    $binders = (1..$n | ForEach-Object { "(a$_ : A) " }) -join ''
    $args = (1..($n - 1) | ForEach-Object { "-> Id A a$_ a$($_ + 1) " }) -join ''
    $ret = "-> Id A a1 a$n"
    $vars = (1..$n | ForEach-Object { "a$_ " }) -join ''
    $refls = (1..($n - 1) | ForEach-Object { 'refl ' }) -join ''
    return "let trans$n : (A : U) $binders$args$ret = \ A $vars$refls. refl; U"
}
foreach ($n in 10, 20, 30, 40, 50) {
    Invoke-Probe -Name "d_trans_$n" -Category exponential -Args @('nf') -Text (New-Trans $n) -TimeoutSec 120 `
        -Note "transN n=$n"
}

# ---- many refl patterns in a row (substitution bookkeeping) ----
foreach ($n in 5, 8, 12) {
    $binders = (1..$n | ForEach-Object { "(a$_ : Nat) " }) -join ''
    $args = (1..($n - 1) | ForEach-Object { "-> Id Nat a$_ a$($_ + 1) " }) -join ''
    $vars = (1..$n | ForEach-Object { "a$_ " }) -join ''
    $refls = (1..($n - 1) | ForEach-Object { 'refl ' }) -join ''
    $src = "let f : $binders$args-> Nat = \ $vars$refls. zero; U"
    Invoke-Probe -Name "d_reflchain_$n" -Category exponential -Args @('nf') -Text $src -TimeoutSec 120 `
        -Note "chain of $($n - 1) refl patterns"
}

# ---- deep but legal suc towers that must still succeed (post-fix expectation) ----
Invoke-Probe -Name d_suc_ok_400 -Category depth -Args @('nf') -Text (('suc ' * 400) + 'zero') -TimeoutSec 30 -Note "regression guard: 400 sucs"
Invoke-Probe -Name d_paren_ok_500 -Category depth -Args @('nf') -Text (('(' * 500) + 'U' + (')' * 500)) -TimeoutSec 30 -Note "regression guard: 500 parens"

Save-Results -Path (Join-Path $script:Root 'results_depth.json')
