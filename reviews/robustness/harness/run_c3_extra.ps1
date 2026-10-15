# Extra category 3 (resource) and category 6 (error position) probes.
# ASCII only (PS 5.1 reads .ps1 as ANSI); non-ASCII is built with [char].
. "$PSScriptRoot\common.ps1"

# ------------------------------------------------ category 6: multi-byte columns
Invoke-Probe -Name e_pos_ascii_arrow -Category position -Args @('nf') -Text "U -> x" -Note "unbound x after ASCII arrow; expect 1:6"
Invoke-Probe -Name e_pos_uni_arrow -Category position -Args @('nf') -Text ("U " + [char]0x2192 + " x") -Note "unbound x after U+2192 arrow; expect 1:5 if columns count chars"
Invoke-Probe -Name e_pos_lambda_uni -Category position -Args @('nf') -Text ([char]0x03BB + " x . y") -Note "unbound y after unicode lambda; expect 1:7"
Invoke-Probe -Name e_pos_accent_line2 -Category position -Args @('nf') -Text ("let " + [char]0x00E9 + " : Nat = zero;`nx") -Note "unbound x on line 2 col 1 after an accented identifier"
Invoke-Probe -Name e_pos_cjk_line2 -Category position -Args @('nf') -Text ("let " + [char]0x65E5 + " : Nat = zero;`n  x") -Note "unbound x line 2 col 3 after a CJK identifier"
Invoke-Probe -Name e_pos_nbsp_line2 -Category position -Args @('nf') -Text ("U`n" + [char]0x00A0 + "x") -Note "NBSP counts as whitespace: x on line 2 col 2"
Invoke-Probe -Name e_pos_line3_nested -Category position -Args @('nf') -Text "let a : Nat = zero;`nlet b : Nat = zero;`nplus a" -Note "plus under-applied on line 3"
Invoke-Probe -Name e_pos_line3_paren -Category position -Args @('nf') -Text "let a : Nat = zero;`nlet b : Nat = zero;`n(a" -Note "unterminated paren on line 3"
Invoke-Probe -Name e_pos_line2_at -Category position -Args @('nf') -Text "U`n@" -Note "bad token on line 2 col 1"
Invoke-Probe -Name e_pos_line5_col7 -Category position -Args @('nf') -Text "U`nU`nU`nU`nlet z : Nat = U; z" -Note "type error line 5 col 19"
Invoke-Probe -Name e_pos_tab_line2 -Category position -Args @('nf') -Text "U`n`tlet z : Nat = U; z" -Note "tab-indented type error line 2"
Invoke-Probe -Name e_pos_after_comment -Category position -Args @('nf') -Text "U`n{- c -}`nlet z : Nat = U; z" -Note "error on line 3 after a block comment line"

# ------------------------------------------------ category 3: resource scaling
# plus of two 500-deep towers -> 1000-deep normal form
$tower500 = ('suc ' * 500) + 'zero'
Invoke-Probe -Name e_plus_500_500 -Category exponential -Args @('nf') -Text ("plus ($tower500) ($tower500)") -TimeoutSec 120 -Note "plus of two 500 towers (1000-succ result)"
Invoke-Probe -Name e_plus_500_500_type -Category exponential -Args @('type') -Text ("plus ($tower500) ($tower500)") -TimeoutSec 120 -Note "same in type mode"

# wide binder group + wide application spine (no parser-depth cost)
$n = 400
$binders = (1..$n | ForEach-Object { "x$_ " }) -join ''
$args = (1..$n | ForEach-Object { 'zero ' }) -join ''
$src = "let f : (${binders}: Nat) -> Nat = \ $binders. zero; f $args"
Invoke-Probe -Name e_wide_apply_400 -Category exponential -Args @('nf') -Text $src -TimeoutSec 120 -Note "400 binders, applied to 400 args"

# type error on a huge line: error renderer echoes the whole line
$big = 'let z : Nat = U;' + (' ' * 200000) + 'z'
Invoke-Probe -Name e_big_line_typeerror -Category malformed -Args @('nf') -Text $big -TimeoutSec 60 -Note "200KB line with a type error: stderr size?"

# many refl-pattern lambdas printed at once
$n2 = 6
$b2 = (1..$n2 | ForEach-Object { "(a$_ : A) " }) -join ''
$a2 = (1..($n2 - 1) | ForEach-Object { "-> Id A a$_ a$($_ + 1) " }) -join ''
$v2 = (1..$n2 | ForEach-Object { "a$_ " }) -join ''
$r2 = (1..($n2 - 1) | ForEach-Object { 'refl ' }) -join ''
Invoke-Probe -Name e_print_refl_lambda -Category exponential -Args @('nf') `
    -Text ("let t : (A : U) $b2$a2-> Id A a1 a$n2 = \ A $v2$r2. refl; t") -TimeoutSec 60 -Note "print a lambda with 5 refl patterns"

Save-Results -Path (Join-Path $script:Root 'results_extra.json')
