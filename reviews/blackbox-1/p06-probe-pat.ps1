. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
$cases = @(
  # --- is (!) allowed as first pattern? ---
  @{i='pt01'; m='nf'; t='let f : Id Nat (suc zero) zero -> Nat = ' + $L + ' (!); f'},
  @{i='pt02'; m='nf'; t='let f : Id Nat (suc zero) zero -> Nat = ' + $L + ' x (!); f'},
  @{i='pt03'; m='nf'; t='let f : Nat -> Id Nat (suc zero) zero -> Nat = ' + $L + ' (!) x; f'},
  @{i='pt04'; m='nf'; t='let f : Nat -> Id Nat (suc zero) zero -> Nat = ' + $L + ' x (!); f'},
  @{i='pt05'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x y . zero; f'},
  @{i='pt06'; m='type'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x y . zero; f'},
  @{i='pt07'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' refl . zero; f'},
  @{i='pt08'; m='nf'; t='let f : Id Nat zero zero -> Nat = ' + $L + ' refl . zero; f'},
  @{i='pt09'; m='nf'; t='let f : Id Nat zero zero -> Nat = ' + $L + ' refl . zero; f refl'},
  @{i='pt10'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' zero . zero; f'},
  @{i='pt11'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' suc . zero; f'},
  @{i='pt12'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; f'},
  @{i='pt13'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x x . x; f'},
  @{i='pt14'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x x . x; f'},
  @{i='pt15'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x x . x; f zero zero'},
  @{i='pt16'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' x . x; g zero'},
  # --- absurd on an INHABITED type (soundness!) ---
  @{i='ab30'; m='nf'; t='let f : (A : U) -> Id Nat zero zero -> A = ' + $L + ' A (!); f'},
  @{i='ab31'; m='nf'; t='let f : (A : U) -> Id Nat zero zero -> A = ' + $L + ' A (!); f Nat refl'},
  @{i='ab32'; m='type'; t='let f : (A : U) -> Id Nat zero zero -> A = ' + $L + ' A (!); f'},
  @{i='ab33'; m='nf'; t='let f : Nat -> Id Nat zero zero -> Nat = ' + $L + ' x (!); f'},
  @{i='ab34'; m='nf'; t='let f : (A : U) -> Id Nat (suc zero) (suc zero) -> A = ' + $L + ' A (!); f'},
  @{i='ab35'; m='nf'; t='let f : (A : U) -> (Id Nat (suc zero) zero -> A) = ' + $L + ' A (!); f Nat'},
  @{i='ab36'; m='nf'; t='let f : (m n : Nat) -> Id Nat m n -> Nat = ' + $L + ' m n (!); f'},
  @{i='ab37'; m='nf'; t='let f : (m n : Nat) -> Id Nat (suc m) (suc n) -> Nat = ' + $L + ' m n (!); f'},
  @{i='ab38'; m='nf'; t='let f : (m n : Nat) -> Id Nat (suc m) (suc n) -> Nat = ' + $L + ' m n (!); f zero zero'},
  # --- eta ---
  @{i='et01'; m='nf'; t='let eta : (g : Nat -> Nat) -> Id (Nat -> Nat) (' + $L + ' x . g x) g = ' + $L + ' g . refl; eta'},
  @{i='et02'; m='nf'; t='let eta : (A : U)(g : A -> A) -> Id (A -> A) (' + $L + ' x . g x) g = ' + $L + ' A g . refl; eta'},
  @{i='et03'; m='type'; t='let eta : (g : Nat -> Nat) -> Id (Nat -> Nat) (' + $L + ' x . g x) g = ' + $L + ' g . refl; eta'},
  @{i='et04'; m='nf'; t='let eta : (g : Nat -> Nat) -> Id (Nat -> Nat) g (' + $L + ' x . g x) = ' + $L + ' g . refl; eta'},
  # --- H used ---
  @{i='hh01'; m='nf'; t='let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; H zero zero refl refl'},
  @{i='hh02'; m='type'; t='let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; H zero zero refl refl'},
  @{i='hh03'; m='nf'; t='let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; H (suc zero) zero refl refl'},
  @{i='hh04'; m='nf'; t='let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; H zero zero refl'},
  @{i='hh05'; m='nf'; t='let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; H'},
  @{i='hh06'; m='type'; t='let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; H'}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 10 -Tag 'probe-pat') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p06.raw.txt') $raw
$raw
