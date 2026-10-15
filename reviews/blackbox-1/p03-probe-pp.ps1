. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB; $A = [char]0x2192
$cases = @(
  @{i='pp01'; m='nf'; t='succ zero'},
  @{i='pp02'; m='nf'; t='succ'},
  @{i='pp03'; m='nf'; t='Suc zero'},
  @{i='pp04'; m='nf'; t='plus (suc zero) zero'},
  @{i='pp05'; m='nf'; t='Nat -> Nat'},
  @{i='pp06'; m='nf'; t='(A : U) -> A -> A'},
  @{i='pp07'; m='nf'; t='(A : U)(B : U) ' + $A + ' A ' + $A + ' B'},
  @{i='pp08'; m='nf'; t='(A : U)(B : U) -> A -> B'},
  @{i='pp09'; m='nf'; t='Id Nat zero zero'},
  @{i='pp10'; m='nf'; t='(x : Nat) -> Id Nat x x'},
  @{i='pp11'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . suc x; f'},
  @{i='pp12'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . suc x; f zero'},
  @{i='pp13'; m='nf'; t='let f : (A : U) -> A -> A = ' + $L + ' A x . x; f'},
  @{i='pp14'; m='nf'; t='let f : (A : U) -> A -> A = ' + $L + ' A x . x; f Nat zero'},
  @{i='pp15'; m='type'; t='let f : (A : U) -> A -> A = ' + $L + ' A x . x; f'},
  @{i='pp16'; m='nf'; t='let f : (A B : U)(g : A -> B)(x : A) -> B = ' + $L + ' A B g x . g x; f Nat Nat (' + $L + ' n . suc n) zero'},
  @{i='pp17'; m='nf'; t='let f : (A B : U)(g : A -> B)(x : A) -> B = ' + $L + ' A B g x . g x; f'},
  @{i='pp18'; m='nf'; t='(Nat -> Nat) -> Nat'},
  @{i='pp19'; m='nf'; t='Id (Nat -> Nat) (' + $L + ' x . x) (' + $L + ' y . y)'},
  @{i='pp20'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' y . y; Id (Nat -> Nat) f g'},
  @{i='pp21'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' y . y; refl'},
  @{i='pp22'; m='nf'; t='Id Nat zero (plus zero zero)'},
  @{i='pp23'; m='type'; t='Id Nat zero (plus zero zero)'},
  @{i='pp24'; m='nf'; t='refl : Id Nat zero zero'},
  @{i='pp25'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' x . x; Id (Nat -> Nat) f g'},
  @{i='pp26'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' x . x; refl'},
  @{i='pp27'; m='nf'; t='let y : Nat = zero; (' + $L + ' x . x) zero'},
  @{i='pp28'; m='nf'; t='let id : (A : U) -> A -> A = ' + $L + ' A x . x; let y : Nat = id Nat zero; y'},
  @{i='pp29'; m='nf'; t='Id Nat (suc zero) (plus zero (suc zero))'},
  @{i='pp30'; m='nf'; t='Id U Nat Nat'}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 10 -Tag 'probe-pp') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p03.raw.txt') $raw
$raw
