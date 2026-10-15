. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
$cases = @(
  @{i='v01'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x x . x; f (suc zero) (suc (suc zero))'},
  @{i='v02'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x y . x; f (suc zero) (suc (suc zero))'},
  @{i='v03'; m='nf'; t='let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = ' + $L + ' A a b c refl refl . refl;' + "`n" + 'let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl;' + "`n" + 'let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!);' + "`n" + 'U'},
  @{i='v04'; m='type'; t='let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = ' + $L + ' A a b c refl refl . refl;' + "`n" + 'let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl;' + "`n" + 'U'},
  @{i='v05'; m='nf'; t='let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; H zero'},
  @{i='v06'; m='nf'; t='let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); C Nat'},
  @{i='v07'; m='nf'; t='let f : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); f'},
  @{i='v08'; m='type'; t='let f : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); f Nat'},
  @{i='v09'; m='nf'; t='let p : Id Nat zero (plus zero zero) = refl; p'},
  @{i='v10'; m='type'; t='let f : Id Nat zero (plus zero zero) -> Nat = ' + $L + ' refl . zero; f'},
  @{i='v11'; m='nf'; t='let f : Id Nat zero (plus zero zero) -> Nat = ' + $L + ' refl . zero; f'},
  @{i='v12'; m='nf'; t='{- a -} {- b -} zero'},
  @{i='v13'; m='nf'; t='{- a {- b -} -} zero'},
  @{i='v14'; m='nf'; t='{- a -} b'},
  @{i='v15'; m='nf'; t='let a : Nat = zero; ' + $L + ' a . a'},
  @{i='v16'; m='nf'; t='let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = ' + $L + ' A a b c refl refl . refl; trans U Nat Nat Nat refl refl'},
  @{i='v17'; m='type'; t='let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = ' + $L + ' A a b c refl refl . refl; trans U Nat Nat Nat refl refl'},
  @{i='v18'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' x . f x; Id (Nat -> Nat) g f'},
  @{i='v19'; m='nf'; t='plus (suc (suc (suc (suc (suc (suc zero)))))) (suc (suc (suc (suc (suc (suc zero))))))'},
  @{i='v20'; m='nf'; t='let p : Id Nat (plus (suc (suc (suc zero))) (suc (suc (suc zero)))) (suc (suc (suc (suc (suc (suc zero)))))) = refl; p'}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 10 -Tag 'verify') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p10.raw.txt') $raw
$raw
