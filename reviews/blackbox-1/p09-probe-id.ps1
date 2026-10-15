. (Join-Path $PSScriptRoot 'lib.ps1')
$L = [char]0x03BB
$cases = @(
  # identifiers charset
  @{i='id01'; m='nf'; t='let a1 : Nat = zero; a1'},
  @{i='id02'; m='nf'; t='let a_b : Nat = zero; a_b'},
  @{i='id03'; m='nf'; t='let _a : Nat = zero; _a'},
  @{i='id04'; m='nf'; t='let aB : Nat = zero; aB'},
  @{i='id05'; m='nf'; t='let abc123 : Nat = zero; abc123'},
  @{i='id06'; m='nf'; t='let a' + [char]0x27 + ' : Nat = zero; a' + [char]0x27},
  @{i='id07'; m='nf'; t='let A' + [char]0x27 + ' : Nat = zero; zero'},
  @{i='id08'; m='nf'; t='let a-b : Nat = zero; a-b'},
  @{i='id09'; m='nf'; t='let a.b : Nat = zero; a.b'},
  @{i='id10'; m='nf'; t='let A1 : Nat = zero; A1'},
  # wildcard
  @{i='wc01'; m='nf'; t='let _ : Nat = zero; _'},
  @{i='wc02'; m='nf'; t='let _ : Nat = zero; let y : Nat = zero; y'},
  @{i='wc03'; m='nf'; t='let f : Nat -> Nat = ' + $L + ' x _ . x; f'},
  @{i='wc04'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' x _ . x; f'},
  @{i='wc05'; m='nf'; t='let f : Nat -> Nat -> Nat = ' + $L + ' _ y . y; f'},
  @{i='wc06'; m='nf'; t='let f : Nat -> Id Nat (suc zero) zero -> Nat = ' + $L + ' x _ . x; f'},
  @{i='wc07'; m='nf'; t='let f : Nat -> Id Nat (suc zero) zero -> Nat = ' + $L + ' x y (!) . x; f'},
  @{i='wc08'; m='nf'; t='let f : Nat -> Nat -> Id Nat (suc zero) zero -> Nat = ' + $L + ' x y (!) . x; f'},
  @{i='wc09'; m='nf'; t='let f : Nat -> Nat -> Id Nat (suc zero) zero -> Nat = ' + $L + ' x (!) y . x; f'},
  @{i='wc10'; m='nf'; t='let f : Id Nat (suc zero) zero -> Nat = ' + $L + ' (!) . zero; f'},
  @{i='wc11'; m='nf'; t='let f : Id Nat (suc zero) zero -> Nat = ' + $L + ' _ . zero; f'},
  @{i='wc12'; m='nf'; t='let f : Id Nat (suc zero) zero -> Id Nat (suc zero) zero -> Nat = ' + $L + ' (!) (!) . zero; f'},
  @{i='wc13'; m='nf'; t='let f : Nat -> Id Nat (suc zero) zero -> Nat = ' + $L + ' x (!) . x; f zero'},
  # CLI args
  @{i='cl01'; m='';         t='zero'},
  @{i='cl02'; m='--help';   t='zero'},
  @{i='cl03'; m='-h';       t='zero'},
  @{i='cl04'; m='NF';       t='zero'},
  @{i='cl05'; m='nf extra'; t='zero'},
  @{i='cl06'; m='nf';       t='zero'},
  @{i='cl07'; m='type';     t='zero'},
  @{i='cl08'; m='--version';t='zero'},
  @{i='cl09'; m='help';     t='zero'},
  @{i='cl10'; m='nf';       t='zero'}
)
$res = @(); foreach ($c in $cases) { $res += (Assert-Case -R (Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 10 -Tag 'probe-id') -Kind 'any') }
$raw = ($res | ForEach-Object { Render-Record $_ }) -join ''
Write-Text (Join-Path $script:WORK 'p09.raw.txt') $raw
$raw
