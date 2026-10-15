. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB   # lambda
$A = [char]0x2192   # arrow
$cases = @(
  @{i='fx01'; m='nf';   t='\ x . x'},
  @{i='fx02'; m='nf';   t="($L x . x) Nat"},
  @{i='fx03'; m='type'; t='\ x . x'},
  @{i='fx04'; m='nf';   t="($L x . x) (Id Nat zero zero)"},
  @{i='fx05'; m='nf';   t="($L x . refl) Nat"},
  @{i='fx06'; m='nf';   t="($L x y . x) Nat zero"},
  @{i='fx07'; m='nf';   t="($L A x . x) Nat zero"},
  @{i='fx08'; m='nf';   t="($L x . plus x zero) (suc zero)"},
  @{i='fx09'; m='nf';   t="($L x . plus zero x) (suc zero)"},
  @{i='fx10'; m='nf';   t="$L x . x"},
  @{i='fx11'; m='nf';   t="($L a b c refl refl . refl) Nat zero zero zero refl refl"},
  @{i='fx12'; m='type'; t="($L a b c refl refl . refl) Nat zero zero zero refl refl"},
  @{i='fx13'; m='nf';   t='\ . zero'},
  @{i='fx14'; m='nf';   t='\x.x'},
  @{i='fx15'; m='nf';   t='\ x y . y'},
  @{i='fx16'; m='nf';   t="($L x . zero) U"},
  @{i='fx17'; m='type'; t='let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = ' + $L + ' A a b c refl refl . refl; U'},
  @{i='fx18'; m='nf';   t='let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = ' + $L + ' A a b c refl refl . refl; trans Nat zero zero zero refl refl'},
  @{i='fx19'; m='type'; t='let cong : (A B : U)(f : A -> B)(a b : A) -> Id A a b -> Id B (f a) (f b) = ' + $L + ' A B f a b refl . refl; U'},
  @{i='fx20'; m='nf';   t='let cong : (A B : U)(f : A -> B)(a b : A) -> Id A a b -> Id B (f a) (f b) = ' + $L + ' A B f a b refl . refl; cong Nat Nat (' + $L + ' n . suc n) zero zero refl'},
  @{i='fx21'; m='type'; t='let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; U'},
  @{i='fx22'; m='type'; t='let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); U'},
  @{i='fx23'; m='nf';   t='let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); C Nat refl'},
  @{i='fx24'; m='nf';   t='let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); C'},
  @{i='fx25'; m='nf';   t="($L x . x) U"},
  @{i='fx26'; m='nf';   t="($L x . U) Nat"},
  @{i='fx27'; m='nf';   t="($L x . Nat) zero"},
  @{i='fx28'; m='nf';   t="($L x y . plus x y) (suc zero) (suc (suc zero))"},
  @{i='fx29'; m='nf';   t="($L x . x) ($L y . y)"},
  @{i='fx30'; m='type'; t="($L x . x) ($L y . y)"}
)
$res = @()
foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 10 -Tag 'explore-lambda') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p02.raw.txt') $raw
$raw
