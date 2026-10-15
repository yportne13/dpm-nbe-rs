. (Join-Path $PSScriptRoot 'lib.ps1')
$cases = @(
  @{i='ex01'; m='nf';   t='U'},
  @{i='ex02'; m='nf';   t='Nat'},
  @{i='ex03'; m='type'; t='Nat'},
  @{i='ex04'; m='nf';   t='zero'},
  @{i='ex05'; m='type'; t='zero'},
  @{i='ex06'; m='nf';   t='suc zero'},
  @{i='ex07'; m='nf';   t='suc (suc (suc zero))'},
  @{i='ex08'; m='nf';   t='plus zero zero'},
  @{i='ex09'; m='nf';   t='plus zero (suc zero)'},
  @{i='ex10'; m='nf';   t='plus (suc zero) zero'},
  @{i='ex11'; m='nf';   t='plus (suc (suc zero)) (suc (suc (suc zero)))'},
  @{i='ex12'; m='type'; t='plus zero zero'},
  @{i='ex13'; m='nf';   t='Id Nat zero zero'},
  @{i='ex14'; m='type'; t='Id Nat zero zero'},
  @{i='ex15'; m='nf';   t='refl'},
  @{i='ex16'; m='nf';   t='Id Nat zero zero -> Nat'},
  @{i='ex17'; m='nf';   t='(A : U) -> A -> A'},
  @{i='ex18'; m='type'; t='(A : U) -> A -> A'},
  @{i='ex19'; m='nf';   t='\ (A : U) (x : A) . x'},
  @{i='ex20'; m='nf';   t='\(A : U) (x : A) . x'},
  @{i='ex21'; m='nf';   t=([char]0x03BB) + ' (A : U) (x : A) . x'},
  @{i='ex22'; m='nf';   t='(A : U) ' + [char]0x2192 + ' A ' + [char]0x2192 + ' A'},
  @{i='ex23'; m='nf';   t='let x : Nat = zero; x'},
  @{i='ex24'; m='nf';   t="let x : Nat = zero; let y : Nat = suc x; y"},
  @{i='ex25'; m='nf';   t="let x : Nat = zero; let x : Nat = suc x; x"},
  @{i='ex26'; m='nf';   t="let x : Nat = zero; -- comment`n x"},
  @{i='ex27'; m='nf';   t="{- block {- nested -} comment -} zero"},
  @{i='ex28'; m='nf';   t="zero -- trailing"},
  @{i='ex29'; m='nf';   t="-- only a comment`nzero"},
  @{i='ex30'; m='type'; t='\ (A : U) (x : A) . x'},
  @{i='ex31'; m='nf';   t='(\(A : U) (x : A) . x) Nat zero'},
  @{i='ex32'; m='nf';   t='\ (n : Nat) . plus n zero'},
  @{i='ex33'; m='type'; t='\ (n : Nat) . plus n zero'},
  @{i='ex34'; m='nf';   t='\ (n : Nat) . plus zero n'},
  @{i='ex35'; m='nf';   t='\ (A : U) (a : A) . refl'},
  @{i='ex36'; m='type'; t='\ (A : U) (a : A) . refl'},
  @{i='ex37'; m='nf';   t='Id'},
  @{i='ex38'; m='nf';   t='Id Nat'},
  @{i='ex39'; m='nf';   t='Id Nat zero'},
  @{i='ex40'; m='nf';   t='plus'},
  @{i='ex41'; m='nf';   t='plus zero zero zero'},
  @{i='ex42'; m='nf';   t='suc zero zero'},
  @{i='ex43'; m='nf';   t='refl zero'},
  @{i='ex44'; m='nf';   t='U U'},
  @{i='ex45'; m='nf';   t="let id : (A : U) -> A -> A = \ (A : U) (x : A) . x; id Nat zero"},
  @{i='ex46'; m='nf';   t='let id : (A : U) -> A -> A = \(A : U)(x : A). x; id (Id Nat zero zero) refl'},
  @{i='ex47'; m='nf';   t='(\(A : U)(x : A). x) (Id Nat zero zero) refl'},
  @{i='ex48'; m='nf';   t='plus (plus zero zero) (plus zero (suc zero))'},
  @{i='ex49'; m='nf';   t='(A : U) -> (B : U) -> A -> B -> A'},
  @{i='ex50'; m='nf';   t='\ (A B : U) (a : A) (b : B) . a'}
)
$res = @()
foreach ($c in $cases) {
  $r = Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 10 -Tag 'explore-basic'
  $res += (Assert-Case -R $r -Kind 'any')
}
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p01.raw.txt') $raw
Get-Content (Join-Path $script:WORK 'p01.raw.txt') -Raw
