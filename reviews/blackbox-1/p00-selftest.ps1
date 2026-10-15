. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
$r1 = Assert-Case -R (Run-Case -Id 'sm01' -Mode 'nf' -Text 'plus (suc zero) (suc zero)') -Kind 'exact' -Expect "succ (succ zero)`n  :`nNat`n"
$r2 = Assert-Case -R (Run-Case -Id 'sm02' -Mode 'nf' -Text 'foo') -Kind 'err' -Expect 'unbound variable: foo'
$r3 = Assert-Case -R (Run-Case -Id 'sm03' -Mode 'nf' -Text ('let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); C')) -Kind 'any'
$r4 = Assert-Case -R (Run-Case -Id 'sm04' -Mode 'nf' -Text 'Id Nat zero zero -> Nat') -Kind 'exact' -Expect ("Id Nat zero zero " + [char]0x2192 + " Nat`n  :`nU`n")
$r5 = Assert-Case -R (Run-Case -Id 'sm05' -Mode 'zzz' -Text 'zero') -Kind 'any'
($r1,$r2,$r3,$r4,$r5) | ForEach-Object { Render-Record $_ }
