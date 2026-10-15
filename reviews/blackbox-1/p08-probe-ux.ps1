. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
$n6 = 'suc (suc (suc (suc (suc (suc zero))))))'   # deliberately balanced later
$succ6 = 'suc (suc (suc (suc (suc (suc zero)))))'
$deep = ''
for ($k = 0; $k -lt 500; $k++) { $deep += 'suc (' }
$deep += 'zero'
for ($k = 0; $k -lt 500; $k++) { $deep += ')' }
$cases = @(
  @{i='ux01'; m='nf'; t='let _ : Nat = zero; zero'},
  @{i='ux02'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' _ . zero; f'},
  @{i='ux03'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' _ . zero; f zero'},
  @{i='ux04'; m='nf'; t='_'},
  @{i='ux05'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' _ _ . zero; f'},
  @{i='ux06'; m='nf'; t='let f : Nat -> Id Nat zero zero -> Nat = ' + $L + ' _ _ . zero; f'},
  @{i='ux07'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . zero; let x' + "'" + ' : Nat = zero; x'},
  @{i='ux08'; m='nf'; t='suc suc suc zero'},
  @{i='ux09'; m='nf'; t='suc (suc zero) zero'},
  @{i='ux10'; m='nf'; t='suc plus zero zero'},
  @{i='ux11'; m='nf'; t='plus (suc zero) suc zero'},
  @{i='ux12'; m='nf'; t='suc Id Nat zero zero'},
  @{i='ux13'; m='nf'; t='Id Nat suc zero zero'},
  @{i='ux14'; m='nf'; t='plus zero plus zero zero'},
  @{i='ux15'; m='nf'; t='suc zero suc zero'},
  @{i='ux16'; m='nf'; t='(suc)(zero)'},
  @{i='ux17'; m='nf'; t='(suc zero)'},
  @{i='ux18'; m='nf'; t='plus (suc zero) (zero)'},
  @{i='ux19'; m='nf'; t='Id (Nat) (zero) (zero)'},
  @{i='ux20'; m='nf'; t='let id : (A : U) -> A -> A = ' + $L + ' A x . x; id _ zero'},
  @{i='ux21'; m='nf'; t=$deep + ' : Nat'},
  @{i='ux22'; m='nf'; t='let p : Id Nat ' + $deep + ' ' + $deep + ' = refl; p'},
  @{i='sc01'; m='nf'; t='plus (' + $succ6 + ') (' + $succ6 + ')'},
  @{i='sc02'; m='nf'; t='plus (' + $deep.Replace('suc','suc') + ') zero'},
  @{i='sc03'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; f (f (f (f (f zero))))'},
  @{i='sc04'; m='nf'; t='plus (suc zero) (plus (suc zero) (plus (suc zero) (plus (suc zero) (plus (suc zero) (plus (suc zero) zero)))))'}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 20 -Tag 'probe-ux') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p08.raw.txt') $raw
$raw
