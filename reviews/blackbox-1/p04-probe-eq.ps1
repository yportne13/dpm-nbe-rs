. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
function EQ([string]$x,[string]$y) { "let p : Id Nat ($x) ($y) = refl; p" }
$cases = @(
  @{i='eq01'; m='nf'; t=(EQ 'plus zero zero' 'zero')},
  @{i='eq02'; m='nf'; t=(EQ 'plus (suc zero) zero' 'suc zero')},
  @{i='eq03'; m='nf'; t=(EQ 'plus zero (suc zero)' 'suc zero')},
  @{i='eq04'; m='nf'; t=(EQ 'plus (suc zero) (suc zero)' 'suc (suc zero)')},
  @{i='eq05'; m='nf'; t=(EQ 'zero' 'plus zero zero')},
  @{i='eq06'; m='nf'; t=(EQ 'plus (plus zero zero) zero' 'zero')},
  @{i='eq07'; m='nf'; t='let x : Nat = zero; let y : Nat = x; let p : Id Nat y x = refl; p'},
  @{i='eq08'; m='nf'; t='let x : Nat = plus zero zero; let p : Id Nat x zero = refl; p'},
  @{i='eq09'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . plus x zero; let p : Id Nat (f zero) zero = refl; p'},
  @{i='eq10'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . plus x zero; let p : Id Nat (f (f zero)) zero = refl; p'},
  @{i='eq11'; m='nf'; t=(EQ 'plus (suc zero) (suc zero)' 'suc (plus (suc zero) zero)')},
  @{i='eq12'; m='nf'; t=(EQ 'plus zero (plus zero zero)' 'plus (plus zero zero) zero')},
  @{i='eq13'; m='nf'; t=(EQ 'suc zero' 'plus zero (suc zero)')},
  @{i='eq14'; m='nf'; t=(EQ 'zero' 'suc zero')},
  @{i='eq15'; m='nf'; t=(EQ 'plus zero (suc zero)' 'zero')},
  @{i='eq16'; m='nf'; t=(EQ 'plus (suc zero) zero' 'zero')},
  @{i='eq17'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' y . y; let p : Id (Nat -> Nat) f g = refl; p'},
  @{i='eq18'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x . plus zero x; let g : Nat -> Nat = ' + $L + ' x . x; let p : Id (Nat -> Nat) f g = refl; p'},
  @{i='eq19'; m='nf'; t='let p : Id (Id Nat zero zero) refl refl = refl; p'},
  @{i='eq20'; m='nf'; t=(EQ 'plus (suc (suc zero)) (suc (suc zero))' 'suc (suc (suc (suc zero)))')},
  @{i='eq21'; m='nf'; t=(EQ 'plus (plus (suc zero) (suc zero)) zero' 'suc (suc zero)')},
  @{i='eq22'; m='nf'; t='let x : Nat = zero; let p : Id Nat (plus x (suc zero)) (suc x) = refl; p'},
  @{i='eq23'; m='nf'; t='let x : Nat = zero; let p : Id Nat (plus (suc zero) x) (suc zero) = refl; p'},
  @{i='eq24'; m='nf'; t='let p : Id U Nat (Nat) = refl; p'},
  @{i='eq25'; m='nf'; t='let p : Id Nat (plus zero zero) (plus zero (plus zero zero)) = refl; p'},
  @{i='eq26'; m='type'; t=(EQ 'plus zero zero' 'zero')},
  @{i='eq27'; m='nf'; t='let p : Id Nat (suc (plus zero zero)) (plus (suc zero) zero) = refl; p'},
  @{i='eq28'; m='nf'; t=(EQ 'plus zero zero' 'plus zero zero')},
  @{i='eq29'; m='nf'; t=(EQ 'suc (suc zero)' 'plus (suc zero) (suc zero)')},
  @{i='eq30'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x y . plus x y; let p : Id Nat (f zero (suc zero)) (suc zero) = refl; p'},
  @{i='eq31'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x y . plus x y; let p : Id Nat (f (suc zero) zero) (suc zero) = refl; p'},
  @{i='eq32'; m='nf'; t='let p : Id Nat zero (plus zero (plus zero zero)) = refl; p'},
  @{i='eq33'; m='nf'; t=(EQ 'plus zero (suc (suc zero))' 'suc (suc zero)')},
  @{i='eq34'; m='nf'; t=(EQ 'plus (suc (suc zero)) zero' 'suc (suc zero)')},
  @{i='eq35'; m='nf'; t='let p : Id Nat zero (plus zero zero) = refl; let q : Id Nat (plus zero zero) zero = refl; q'}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 10 -Tag 'probe-eq') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p04.raw.txt') $raw
$raw
