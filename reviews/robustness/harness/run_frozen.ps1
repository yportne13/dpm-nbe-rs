# Verification against the FROZEN binary (SHA-256 8A769532...).  ASCII only.
# Focus: the five open findings F1-F5 plus regression guards.
. "$PSScriptRoot\common.ps1"

# ---- F1: keyword parse errors must point at the token start ----
Invoke-Probe -Name f_kw_suc_let  -Category position -Args @('nf') -Text "suc let"       -Note "expect 1:5 (was 1:8)"
Invoke-Probe -Name f_kw_in       -Category position -Args @('nf') -Text "U in"          -Note "expect 1:3 (was 1:5)"
Invoke-Probe -Name f_kw_plus_let -Category position -Args @('nf') -Text "plus zero let" -Note "expect 1:11 (was 1:14)"
Invoke-Probe -Name f_kw_let_only -Category position -Args @('nf') -Text "let"           -Note "bare let"
Invoke-Probe -Name f_kw_suc_nat  -Category position -Args @('nf') -Text "suc Nat"       -Note "control: type error at 1:5"
Invoke-Probe -Name f_kw_spine    -Category position -Args @('nf') -Text "U let"         -Note "keyword after a complete atom"

# ---- F2: BOM must be skipped ----
Invoke-Probe -Name f_bom      -Category malformed -Args @('nf') -Bytes ([byte[]](0xEF,0xBB,0xBF,0x55,0x0A)) -Note "BOM + U: expect exit 0"
Invoke-Probe -Name f_bom_crlf -Category malformed -Args @('nf') -Bytes ([byte[]](0xEF,0xBB,0xBF,0x4E,0x61,0x74,0x0D,0x0A)) -Note "BOM + Nat CRLF: expect exit 0"
Invoke-Probe -Name f_bom_type -Category malformed -Args @('type') -Bytes ([byte[]](0xEF,0xBB,0xBF,0x55,0x0A)) -Note "BOM in type mode"
Invoke-Probe -Name f_bom_mid  -Category malformed -Args @('nf') -Bytes ([byte[]](0x55,0xEF,0xBB,0xBF,0x0A)) -Note "BOM after U (not leading): expect clean error"

# ---- F3: long-line echo must be windowed ----
Invoke-Probe -Name f_long_ident      -Category malformed -Args @('nf') -Text ('a' * 100000) -Note "100k identifier"
Invoke-Probe -Name f_long_comment    -Category malformed -Args @('nf') -Text ('{-' + ('-' * 200000)) -Note "200k unterminated comment"
Invoke-Probe -Name f_long_typeerr    -Category malformed -Args @('nf') -Text ('let z : Nat = U;' + (' ' * 200000) + 'z') -Note "200k-char line with a type error"
Invoke-Probe -Name f_long_zero_spine -Category malformed -Args @('nf') -Text ('zero ' * 100000) -Note "flat zero spine x100000"
Invoke-Probe -Name f_long_mid_error  -Category malformed -Args @('nf') -Text (('a' * 50000) + ' U') -Note "error in the middle of a 50k-char line"

# ---- F4: tab caret (documented as matching the original) ----
Invoke-Probe -Name f_tab_caret -Category position -Args @('nf') -Text "let ok : Nat = zero;`n`tlet bad : Nat = U;`nU" -Note "type error 2:18, caret tab=1"
Invoke-Probe -Name f_tab_multi -Category position -Args @('nf') -Text "`t`tU`n`t`tlet z : Nat = U; z" -Note "two tabs before the error line"

# ---- F5: depth policy (MAX_DEPTH = 512) + sticky message ----
foreach ($n in 255, 256) { Invoke-Probe -Name "f_paren_$n" -Category depth -Args @('nf') -Text (('(' * $n) + 'U' + (')' * $n)) -TimeoutSec 30 }
foreach ($n in 500, 511) { Invoke-Probe -Name "f_suc_$n" -Category depth -Args @('nf') -Text (('suc ' * $n) + 'zero') -TimeoutSec 30 }
foreach ($n in 500, 511) { Invoke-Probe -Name "f_arrow_$n" -Category depth -Args @('nf') -Text ((@('Nat') + (1..$n | ForEach-Object { '-> Nat' })) -join ' ') -TimeoutSec 30 }
foreach ($n in 500, 1000) { Invoke-Probe -Name "f_lets_$n" -Category depth -Args @('nf') -Text (((1..$n | ForEach-Object { "let x$_ : Nat = zero; " }) -join '') + 'U') -TimeoutSec 30 }
foreach ($n in 50000, 100000) { Invoke-Probe -Name "f_spineU_$n" -Category depth -Args @('nf') -Text ('U ' * $n) -TimeoutSec 60 }
Invoke-Probe -Name f_suc_10000 -Category depth -Args @('nf') -Text (('suc ' * 10000) + 'zero') -TimeoutSec 30 -Note "10k suc tower: depth error expected"

# ---- regression guards ----
Invoke-Probe -Name f_print_absurd -Category panic -Args @('nf') -Text 'let C : (A : U) -> Id Nat (suc zero) zero -> A = \ A (!); C' -Note "print absurd lambda"
Invoke-Probe -Name f_absurd_index -Category panic -Args @('nf') -Text 'let C : (A : U) -> Id Nat (suc zero) zero -> A = \ A (!); Id ((A : U) -> Id Nat (suc zero) zero -> A) C C' -Note "absurd lambda as Id index"
Invoke-Probe -Name f_absurd_conv  -Category panic -Args @('nf') -Text 'let T : U = (A : U) -> Id Nat (suc zero) zero -> A;`nlet h : Id T (\ A (!)) (\ A (!)) = refl;`nU' -Note "conv of two absurd lambdas"
Invoke-Probe -Name f_nonutf8      -Category malformed -Args @('nf') -Bytes ([byte[]](0x55,0xFF,0xFE,0x0A)) -Note "invalid UTF-8: expect exit 1"
Invoke-Probe -Name f_unterm       -Category malformed -Args @('nf') -Text "U {- never closed" -Note "unterminated comment: expect exit 1"
Invoke-Probe -Name f_empty        -Category malformed -Args @('nf') -Text "" -Note "empty stdin: expect exit 1"
$t500 = ('suc ' * 500) + 'zero'
Invoke-Probe -Name f_plus_500 -Category exponential -Args @('nf') -Text ("plus ($t500) ($t500)") -TimeoutSec 30
$nb = 400
$fBinders = (1..$nb | ForEach-Object { "x$_ " }) -join ''
$fArgs = (1..$nb | ForEach-Object { 'zero ' }) -join ''
Invoke-Probe -Name f_wide_apply -Category exponential -Args @('nf') -Text ("let f : (${fBinders}: Nat) -> Nat = \ $fBinders. zero; f $fArgs") -TimeoutSec 30

function New-Trans5([int]$n) {
    $b = (1..$n | ForEach-Object { "(a$_ : A) " }) -join ''
    $ar = (1..($n - 1) | ForEach-Object { "-> Id A a$_ a$($_ + 1) " }) -join ''
    $v = (1..$n | ForEach-Object { "a$_ " }) -join ''
    $rf = (1..($n - 1) | ForEach-Object { 'refl ' }) -join ''
    return "let t : (A : U) $b$ar-> Id A a1 a$n = \ A $v$rf. refl; U"
}
foreach ($n in 10, 100, 200) { Invoke-Probe -Name "f_trans_$n" -Category exponential -Args @('nf') -Text (New-Trans5 $n) -TimeoutSec 120 }

Save-Results -Path (Join-Path $script:Root 'results_frozen.json')
