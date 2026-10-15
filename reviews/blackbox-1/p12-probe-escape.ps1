. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
$cases = @(
  # attempts to escape from an assumed equation (these MUST be rejected)
  @{i='s01'; m='nf'; t='let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat (plus m zero) zero = ' + $L + ' m refl . refl; f'},
  @{i='s02'; m='nf'; t='let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat m zero = ' + $L + ' m refl . refl; f'},
  @{i='s03'; m='nf'; t='let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat (suc zero) zero = ' + $L + ' m refl . refl; f'},
  @{i='s04'; m='nf'; t='let f : (m : Nat) -> Id Nat m (suc m) -> Nat = ' + $L + ' m refl . zero; f'},
  @{i='s05'; m='nf'; t='let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat zero (suc zero) = ' + $L + ' m refl . refl; f'},
  @{i='s06'; m='nf'; t='let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat (suc zero) zero = ' + $L + ' m refl . refl; f'},
  @{i='s07'; m='nf'; t='let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat m (suc (suc m)) = ' + $L + ' m refl . refl; f'},
  @{i='s08'; m='nf'; t='let p : Id Nat (suc zero) zero = refl; p'},
  @{i='s09'; m='nf'; t='let f : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); let bad : Nat = f Nat; bad'},
  @{i='s10'; m='nf'; t='let f : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); let bad : Id Nat zero (suc zero) = f (Id Nat zero (suc zero)); bad'},
  @{i='s11'; m='type'; t='let f : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); let bad : Id Nat zero (suc zero) = f (Id Nat zero (suc zero)); bad'},
  @{i='s12'; m='nf'; t='let f : (m : Nat) -> Id Nat m (suc m) -> Nat = ' + $L + ' m refl . zero; f zero refl'},
  @{i='s13'; m='nf'; t='let f : (m : Nat) -> Id Nat m (suc m) -> Nat = ' + $L + ' m refl . zero; f (suc zero) refl'},
  @{i='s14'; m='nf'; t='let f : (m : Nat) -> Id Nat (plus m zero) m -> Nat = ' + $L + ' m refl . zero; f (suc zero) refl'},
  @{i='s15'; m='nf'; t='let f : (m : Nat) -> Id Nat (plus m zero) m -> Nat = ' + $L + ' m refl . zero; f zero refl'}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 20 -Tag 'probe-sound2') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p12.raw.txt') $raw
$raw
