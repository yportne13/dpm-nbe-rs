. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
$cases = @(
  # --- soundness: must FAIL (plus n zero does not reduce for neutral n) ---
  @{i='sd01'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . plus x zero; let g : Nat -> Nat = ' + $L + ' x . x; let p : Id (Nat -> Nat) f g = refl; p'},
  @{i='sd02'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . plus x (suc zero); let g : Nat -> Nat = ' + $L + ' x . suc x; let p : Id (Nat -> Nat) f g = refl; p'},
  @{i='sd03'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . plus (suc zero) x; let g : Nat -> Nat = ' + $L + ' x . suc x; let p : Id (Nat -> Nat) f g = refl; p'},
  @{i='sd04'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . plus zero x; let g : Nat -> Nat = ' + $L + ' x . x; let p : Id (Nat -> Nat) f g = refl; p'},
  # --- eta ---
  @{i='sd05'; m='nf'; t='let g : Nat -> Nat = ' + $L + ' x . x; let f : Nat -> Nat = ' + $L + ' x . g x; let p : Id (Nat -> Nat) f g = refl; p'},
  @{i='sd06'; m='nf'; t='let g : Nat -> Nat = ' + $L + ' x . x; let p : Id (Nat -> Nat) g (' + $L + ' x . g x) = refl; p'},
  # --- multi-arity alpha ---
  @{i='sd07'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x y . x; let g : Nat -> Nat -> Nat = ' + $L + ' a . ' + $L + ' b . a; let p : Id (Nat -> Nat -> Nat) f g = refl; p'},
  @{i='sd08'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x y . x; let g : Nat -> Nat -> Nat = ' + $L + ' y x . x; let p : Id (Nat -> Nat -> Nat) f g = refl; p'},
  # --- dependent function equality ---
  @{i='sd09'; m='nf'; t='let f : (A : U) -> A -> A = ' + $L + ' A x . x; let g : (A : U) -> A -> A = ' + $L + ' B y . y; let p : Id ((A : U) -> A -> A) f g = refl; p'},
  @{i='sd10'; m='nf'; t='let f : (A : U) -> A -> A = ' + $L + ' A x . x; let g : (A : U) -> A -> A = ' + $L + ' A x . g A x; let p : Id ((A : U) -> A -> A) f g = refl; p'},
  # --- Id of types: distinct types must not be convertible ---
  @{i='sd11'; m='nf'; t='let p : Id U Nat (Nat -> Nat) = refl; p'},
  @{i='sd12'; m='nf'; t='let g : Nat -> Nat = ' + $L + ' x . x; let p : Id U Nat (Nat -> Nat) = refl; p'},
  # --- absurd: partial application / stuck Bot ---
  @{i='ab01'; m='nf'; t='let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); C Nat'},
  @{i='ab02'; m='nf'; t='let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); C'},
  @{i='ab03'; m='nf'; t='let C : Id Nat (suc zero) zero -> Nat = ' + $L + ' (!); C'},
  @{i='ab04'; m='nf'; t='let C : Id Nat (suc zero) zero -> Nat = ' + $L + ' (!); C'},
  @{i='ab05'; m='nf'; t='let C : Id Nat (suc zero) zero -> Nat = ' + $L + ' (!); C refl'},
  @{i='ab06'; m='nf'; t='let C : Id Nat zero zero -> Nat = ' + $L + ' (!); C'},
  @{i='ab07'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' (!); f'},
  @{i='ab08'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' (!); f zero'},
  @{i='ab09'; m='nf'; t='let f : U -> Nat = ' + $L + ' (!); f'},
  @{i='ab10'; m='nf'; t=$L + ' (!) . zero'},
  @{i='ab11'; m='nf'; t='let f : Id Nat (suc zero) zero -> Nat = ' + $L + ' (!); f refl'},
  @{i='ab12'; m='nf'; t='let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); C Nat refl'},
  @{i='ab13'; m='type'; t='let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); C Nat'},
  @{i='ab14'; m='type'; t='let f : Nat -> Nat = ' + $L + ' (!); f'},
  @{i='ab15'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' (!); f (f zero)'},
  @{i='ab16'; m='nf'; t='let f : (n : Nat) -> Id Nat n zero -> Nat = ' + $L + ' n (!); f'},
  @{i='ab17'; m='nf'; t='let f : (n : Nat) -> Id Nat (suc n) zero -> Nat = ' + $L + ' n (!); f'},
  @{i='ab18'; m='nf'; t='let f : (n : Nat) -> Id Nat (suc n) zero -> Nat = ' + $L + ' n (!); f zero'},
  @{i='ab19'; m='nf'; t='let f : Id Nat (suc zero) (suc zero) -> Nat = ' + $L + ' (!); f'},
  @{i='ab20'; m='nf'; t='let p : Id Nat (suc zero) zero = refl; p'}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 10 -Tag 'probe-sound') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p05.raw.txt') $raw
$raw
