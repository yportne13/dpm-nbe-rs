. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
$cases = @(
  # DPM unification stress: things that are NOT definitionally equal for neutral args
  @{i='u01'; m='nf'; t='let f : (m : Nat) -> Id Nat (plus m zero) m -> Nat = ' + $L + ' m refl . zero; f'},
  @{i='u02'; m='nf'; t='let f : (m : Nat) -> Id Nat (plus m zero) m -> Nat = ' + $L + ' m refl . zero; f zero refl'},
  @{i='u03'; m='nf'; t='let f : (m n : Nat) -> Id Nat (plus m n) (plus n m) -> Nat = ' + $L + ' m n refl . zero; f'},
  @{i='u04'; m='nf'; t='let f : (m n : Nat) -> Id Nat (plus m n) (plus n m) -> Nat = ' + $L + ' m n (!); f'},
  @{i='u05'; m='nf'; t='let f : (m : Nat) -> Id Nat m (plus m zero) -> Nat = ' + $L + ' m refl . zero; f'},
  @{i='u06'; m='nf'; t='let f : (m : Nat) -> Id Nat (suc m) zero -> Nat = ' + $L + ' m refl . zero; f'},
  @{i='u07'; m='nf'; t='let f : (m : Nat) -> Id Nat (suc m) zero -> Nat = ' + $L + ' m (!); f'},
  @{i='u08'; m='nf'; t='let f : (m : Nat) -> Id Nat (suc m) (suc (suc m)) -> Nat = ' + $L + ' m (!); f'},
  @{i='u09'; m='nf'; t='let f : (m n : Nat) -> Id Nat (suc m) (suc n) -> Id Nat m n = ' + $L + ' m n refl . refl; f'},
  @{i='u10'; m='nf'; t='let f : (m n : Nat) -> Id Nat (suc m) (suc n) -> Id Nat m n = ' + $L + ' m n refl . refl; f zero zero refl'},
  @{i='u11'; m='nf'; t='let f : (m n : Nat) -> Id Nat (suc m) (suc n) -> Id Nat m n = ' + $L + ' m n refl . refl; f (suc zero) (suc zero) refl'},
  @{i='u12'; m='nf'; t='let f : (m n : Nat) -> Id Nat (suc m) (suc n) -> Id Nat m n = ' + $L + ' m n refl . refl; f zero (suc zero) refl'},
  @{i='u13'; m='nf'; t='let f : Id Nat (plus zero zero) zero -> Nat = ' + $L + ' refl . zero; f refl'},
  @{i='u14'; m='nf'; t='let f : (m : Nat) -> Id Nat zero (plus m zero) -> Nat = ' + $L + ' m refl . zero; f'},
  # type-level let
  @{i='u15'; m='nf'; t='let A : U = Nat; let x : A = zero; x'},
  @{i='u16'; m='nf'; t='let A : U = Id Nat zero zero; let x : A = refl; x'},
  @{i='u17'; m='type'; t='let A : U = Nat; let x : A = zero; x'},
  @{i='u18'; m='nf'; t='let F : U -> U = ' + $L + ' A . A; F Nat'},
  @{i='u19'; m='nf'; t='let F : U -> U = ' + $L + ' A . A; let x : F Nat = zero; x'},
  @{i='u20'; m='nf'; t='let f : Nat -> U = ' + $L + ' n . Nat; let x : f zero = zero; x'},
  # printer/parser edges
  @{i='u21'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' y . y; Id (Nat -> Nat) f g'},
  @{i='u22'; m='nf'; t='let f : (A : U) -> A -> A = ' + $L + ' A x . x; let g : (A : U) -> A -> A = ' + $L + ' A x . x; Id ((A : U) -> A -> A) f g'},
  @{i='u23'; m='nf'; t='let f : (A B : U) -> A -> B -> A = ' + $L + ' A B a b . a; f'},
  @{i='u24'; m='nf'; t='let f : (n : Nat) -> Nat = ' + $L + ' n . n; let g : (n : Nat) -> Nat = ' + $L + ' m . m; Id ((n : Nat) -> Nat) f g'},
  # deep / wide smoke tests
  @{i='u25'; m='nf'; t=('let f : Nat -> Nat = ' + $L + ' x . x; ' + ((1..25 | ForEach-Object { 'f (' }) -join '') + 'zero' + ((1..25 | ForEach-Object { ')' }) -join ''))}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 20 -Tag 'probe-final') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p11.raw.txt') $raw
$raw
