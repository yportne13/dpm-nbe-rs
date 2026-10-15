. (Join-Path $PSScriptRoot 'lib.ps1')
$L  = [char]0x03BB
$AR = [char]0x2192
$NL = "`n"

function NF([string]$t,[string]$ty) { "$t`n  :`n$ty`n" }
function Num([int]$n) { if ($n -le 0) { 'zero' } else { 'suc (' + (Num ($n-1)) + ')' } }
# printer output: succ zero, succ (succ zero), succ (succ (succ zero)) ...
function NumNF([int]$n) {
  if ($n -le 0) { return 'zero' }
  if ($n -eq 1) { return 'succ zero' }
  return 'succ (' + (NumNF ($n-1)) + ')'
}
function Jet([string]$n) { $n -replace 'succ','suc' }
$LAM = '@LAM@'   # placeholder: this file must stay pure ASCII for PowerShell 5.1

$README_RUST = (@'
let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  @LAM@ A a b c refl refl . refl;

let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  @LAM@ m n refl refl . refl;

let C : (A : U) -> Id Nat (suc zero) zero -> A =
  @LAM@ A (!);   -- absurd pattern

U
'@).Replace($LAM, $L)

$README_ROOT = (@'
let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  @LAM@ A a b c refl refl . refl; 

let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) =
  @LAM@ A B f a b refl . refl;

let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  @LAM@ m n refl refl . refl;

let C : (A : U) -> Id Nat (suc zero) zero -> A = 
  @LAM@ A (!); -- Absurd pattern

U
'@).Replace($LAM, $L)

$TRANS = 'let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = ' + $L + ' A a b c refl refl . refl; '
$CONG  = 'let cong : (A B : U)(f : A -> B)(a b : A) -> Id A a b -> Id B (f a) (f b) = ' + $L + ' A B f a b refl . refl; '
$H     = 'let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = ' + $L + ' m n refl refl . refl; '
$C     = 'let C : (A : U) -> Id Nat (suc zero) zero -> A = ' + $L + ' A (!); '

# generated transitivity family: n+1 endpoints, n identity hypotheses
function Trans([int]$n) {
  $vars = @('a','b','c','d','e','f','g')
  $as = @()
  for ($k=0; $k -le $n; $k++) { $as += $vars[$k] }
  $binders = '(' + ($as -join ' ') + ' : A)'
  $args = @()
  for ($k=0; $k -lt $n; $k++) { $args += ('Id A ' + $as[$k] + ' ' + $as[$k+1]) }
  $arrow = (($args + ('Id A ' + $as[0] + ' ' + $as[$n])) -join ' -> ')
  $pats = @('A') + $as
  for ($k=0; $k -lt $n; $k++) { $pats += 'refl' }
  ('let trans' + $n + ' : (A : U)' + $binders + ' -> ' + $arrow + ' = ' +
   $L + ' ' + ($pats -join ' ') + ' . refl;')
}
function TransApp([int]$n) {
  $zeros = (1..($n+1) | ForEach-Object { 'zero' }) -join ' '
  $refls = (1..$n | ForEach-Object { 'refl' }) -join ' '
  'trans' + $n + ' Nat ' + $zeros + ' ' + $refls
}

$cases = @()
function Add-Case([string]$id,[string]$tag,[string]$mode,[string]$text,[string]$kind,[string]$expect='') {
  $script:cases += @{ i=$id; tag=$tag; m=$mode; t=$text; k=$kind; e=$expect }
}
function C([string]$id,[string]$tag,[string]$mode,[string]$text,[string]$kind,[string]$expect='') {
  Add-Case $id $tag $mode $text $kind $expect
}

# ---------------- A. basics: numerals, plus, Id, functions, let, shadowing, comments, unicode
C 'A01' 'A-basic' 'nf'   'U'                                  'exact' (NF 'U' 'U')
C 'A02' 'A-basic' 'nf'   'Nat'                                'exact' (NF 'Nat' 'U')
C 'A03' 'A-basic' 'type' 'Nat'                                'exact' "U$NL"
C 'A04' 'A-basic' 'nf'   'zero'                               'exact' (NF 'zero' 'Nat')
C 'A05' 'A-basic' 'type' 'zero'                               'exact' "Nat$NL"
C 'A06' 'A-basic' 'nf'   'suc zero'                           'exact' (NF 'succ zero' 'Nat')
C 'A07' 'A-basic' 'nf'   'suc (suc (suc zero))'               'exact' (NF 'succ (succ (succ zero))' 'Nat')
C 'A08' 'A-basic' 'nf'   'plus zero zero'                     'exact' (NF 'zero' 'Nat')
C 'A09' 'A-basic' 'nf'   'plus zero (suc zero)'               'exact' (NF 'succ zero' 'Nat')
C 'A10' 'A-basic' 'nf'   'plus (suc zero) zero'               'exact' (NF 'succ zero' 'Nat')
C 'A11' 'A-basic' 'nf'   ('plus (' + (Num 2) + ') (' + (Num 3) + ')') 'exact' (NF (NumNF 5) 'Nat')
C 'A12' 'A-basic' 'nf'   'Id Nat zero zero'                   'exact' (NF 'Id Nat zero zero' 'U')
C 'A13' 'A-basic' 'type' 'Id Nat zero zero'                   'exact' "U$NL"
C 'A14' 'A-basic' 'nf'   '(A : U) -> A -> A'                  'exact' (NF ('(A : U) ' + $AR + ' A ' + $AR + ' A') 'U')
C 'A15' 'A-basic' 'type' '(A : U) -> A -> A'                  'exact' "U$NL"
C 'A16' 'A-basic' 'nf'   'Nat -> Nat'                         'exact' (NF ('Nat ' + $AR + ' Nat') 'U')
C 'A17' 'A-basic' 'nf'   ('(A : U) ' + $AR + ' A ' + $AR + ' A') 'exact' (NF ('(A : U) ' + $AR + ' A ' + $AR + ' A') 'U')
C 'A18' 'A-basic' 'nf'   ("-- comment$NL zero")               'exact' (NF 'zero' 'Nat')
C 'A19' 'A-basic' 'nf'   '{- comment -} zero'                 'exact' (NF 'zero' 'Nat')
C 'A20' 'A-basic' 'nf'   '{- a -} {- b -} zero'               'exact' (NF 'zero' 'Nat')
C 'A21' 'A-basic' 'nf'   'zero -- trailing'                   'exact' (NF 'zero' 'Nat')
C 'A22' 'A-basic' 'nf'   'let x : Nat = zero; x'              'exact' (NF 'zero' 'Nat')
C 'A23' 'A-basic' 'nf'   'let x : Nat = zero; let y : Nat = suc x; y' 'exact' (NF 'succ zero' 'Nat')
C 'A24' 'A-basic' 'nf'   'let x : Nat = zero; let x : Nat = suc x; x' 'exact' (NF 'succ zero' 'Nat')
C 'A25' 'A-basic' 'nf'   ('let f : Nat -> Nat = ' + $L + ' x . suc x; f zero') 'exact' (NF 'succ zero' 'Nat')
C 'A26' 'A-basic' 'nf'   ('let f : (A : U) -> A -> A = ' + $L + ' A x . x; f Nat zero') 'exact' (NF 'zero' 'Nat')
C 'A27' 'A-basic' 'nf'   ('let f : (A B : U)(g : A -> B)(x : A) -> B = ' + $L + ' A B g x . g x; f Nat Nat (' + $L + ' n . suc n) zero') 'exact' (NF 'succ zero' 'Nat')
C 'A28' 'A-basic' 'type' ('let f : Nat -> Nat = ' + $L + ' x . suc x; f') 'exact' ('Nat ' + $AR + " Nat$NL")
C 'A29' 'A-basic' 'nf'   'Id U Nat Nat'                       'exact' (NF 'Id U Nat Nat' 'U')
C 'A30' 'A-basic' 'nf'   '(x : Nat) -> Id Nat x x'            'exact' (NF ('(x : Nat) ' + $AR + ' Id Nat x x') 'U')
C 'A31' 'A-basic' 'nf'   '(Nat -> Nat) -> Nat'                'exact' (NF ('(Nat ' + $AR + ' Nat) ' + $AR + ' Nat') 'U')
C 'A32' 'A-basic' 'nf'   '(A : U) -> (A : U) -> A'            'exact' (NF ('(A : U)(A' + [char]0x27 + ' : U) ' + $AR + ' A' + [char]0x27) 'U')
C 'A33' 'A-basic' 'nf'   ('let f : Nat -> Nat -> Nat = ' + $L + ' x x . x; f (suc zero) (suc (suc zero))') 'exact' (NF 'succ (succ zero)' 'Nat')
C 'A34' 'A-basic' 'nf'   ('let f : Nat -> Nat = ' + $L + ' x . x; let g : Nat -> Nat = ' + $L + ' x . x; g zero') 'exact' (NF 'zero' 'Nat')
C 'A35' 'A-basic' 'nf'   'let _ : Nat = zero; zero'           'exact' (NF 'zero' 'Nat')
C 'A36' 'A-basic' 'nf'   'let _ : Nat = zero; _'              'err'   'expected an atom'
C 'A37' 'A-basic' 'nf'   '{- a {- b -} -} zero'               'info'  ''      # nested comments: spec silent

# ---------------- B. README examples + generated transitivity proofs
C 'B01' 'B-readme' 'type' $README_RUST 'exact' "U$NL"
C 'B02' 'B-readme' 'nf'   $README_RUST 'exact' (NF 'U' 'U')
C 'B03' 'B-readme' 'type' $README_ROOT 'exact' "U$NL"
C 'B04' 'B-readme' 'nf'   $README_ROOT 'exact' (NF 'U' 'U')
C 'B05' 'B-readme' 'type' ($TRANS + 'U') 'exact' "U$NL"
C 'B06' 'B-readme' 'nf'   ($TRANS + 'trans Nat zero zero zero refl refl') 'exact' (NF 'refl' 'Id Nat zero zero')
C 'B07' 'B-readme' 'type' ($CONG + 'U') 'exact' "U$NL"
C 'B08' 'B-readme' 'nf'   ($CONG + 'cong Nat Nat (' + $L + ' n . suc n) zero zero refl') 'exact' (NF 'refl' 'Id Nat (succ zero) (succ zero)')
C 'B09' 'B-readme' 'type' ($H + 'U') 'exact' "U$NL"
C 'B10' 'B-readme' 'nf'   ($H + 'H zero zero refl refl') 'exact' (NF 'refl' 'Id Nat zero zero')
C 'B11' 'B-readme' 'type' ($C + 'U') 'exact' "U$NL"
C 'B12' 'B-readme' 'nf'   ($C + 'C Nat refl') 'err' 'expecting succ zero equals to zero'
C 'B13' 'B-readme' 'nf'   ($C + 'C Nat') 'ok' ''
C 'B14' 'B-readme' 'type' ($C + 'C Nat') 'exact' ('Id Nat (succ zero) zero ' + $AR + " Nat$NL")
C 'B15' 'B-gen'   'type' ((Trans 3) + 'U') 'exact' "U$NL"
C 'B16' 'B-gen'   'nf'   ((Trans 3) + (TransApp 3)) 'exact' (NF 'refl' 'Id Nat zero zero')
C 'B17' 'B-gen'   'type' ((Trans 4) + 'U') 'exact' "U$NL"
C 'B18' 'B-gen'   'nf'   ((Trans 4) + (TransApp 4)) 'exact' (NF 'refl' 'Id Nat zero zero')
C 'B19' 'B-gen'   'type' ((Trans 5) + 'U') 'exact' "U$NL"
C 'B20' 'B-gen'   'nf'   ((Trans 5) + (TransApp 5)) 'exact' (NF 'refl' 'Id Nat zero zero')
C 'B21' 'B-gen'   'nf'   ((Trans 2) + 'trans2 U Nat Nat Nat refl refl') 'exact' (NF 'refl' 'Id U Nat Nat')
C 'B22' 'B-gen'   'nf'   ((Trans 2) + 'let f : Nat -> Nat = ' + $L + ' x . x; trans2 (Nat -> Nat) f f f refl refl') 'exact' (NF 'refl' ('Id (Nat ' + $AR + ' Nat) (' + $L + ' x . x) (' + $L + ' x . x)'))
C 'B23' 'B-gen'   'nf'   ((Trans 2) + 'trans2 (Id Nat zero zero) refl refl refl refl refl') 'exact' (NF 'refl' 'Id (Id Nat zero zero) refl refl')
C 'B24' 'B-gen'   'nf'   ((Trans 2) + 'trans2 Nat zero zero zero (trans2 Nat zero zero zero refl refl) refl') 'exact' (NF 'refl' 'Id Nat zero zero')
C 'B25' 'B-gen'   'nf'   ((Trans 2) + 'trans2 Nat zero zero (suc zero) refl refl') 'err' 'expecting'
C 'B26' 'B-gen'   'nf'   ($H + 'H zero (suc zero) refl refl') 'err' 'expecting'
C 'B27' 'B-gen'   'nf'   ($H + 'H (suc zero) (suc zero) refl refl') 'err' 'expecting'
C 'B28' 'B-gen'   'nf'   ($CONG + 'cong Nat Nat (' + $L + ' n . suc n) zero (suc zero) refl') 'err' 'expecting'
C 'B29' 'B-gen'   'nf'   ($H + 'H zero zero refl') 'exact' (NF ('(' + $L + ' refl refl . refl)') 'Id Nat zero zero ' + $AR + ' Id Nat zero zero')
C 'B30' 'B-gen'   'nf'   ($H + 'H zero') 'exact' (NF ('(' + $L + ' n refl refl . refl)') ('(n : Nat) ' + $AR + ' Id Nat n zero ' + $AR + ' Id Nat zero zero ' + $AR + ' Id Nat n zero'))

# ---------------- C. definitional equality (refl must / must not check)
function RE([string]$a,[string]$b) { 'let p : Id Nat (' + $a + ') (' + $b + ') = refl; p' }
C 'C01' 'C-defeq' 'nf' (RE 'plus zero zero' 'zero')             'exact' (NF 'refl' 'Id Nat zero zero')
C 'C02' 'C-defeq' 'nf' (RE 'plus (suc zero) zero' 'suc zero')   'exact' (NF 'refl' 'Id Nat (succ zero) (succ zero)')
C 'C03' 'C-defeq' 'nf' (RE 'plus zero (suc zero)' 'suc zero')   'exact' (NF 'refl' 'Id Nat (succ zero) (succ zero)')
C 'C04' 'C-defeq' 'nf' (RE ('plus (' + (Num 2) + ') (' + (Num 2) + ')') (Num 4)) 'exact' (NF 'refl' ('Id Nat (' + (NumNF 4) + ') (' + (NumNF 4) + ')'))
C 'C05' 'C-defeq' 'nf' (RE 'zero' 'plus zero zero')            'exact' (NF 'refl' 'Id Nat zero zero')
C 'C06' 'C-defeq' 'nf' (RE 'plus (plus zero zero) zero' 'zero') 'exact' (NF 'refl' 'Id Nat zero zero')
C 'C07' 'C-defeq' 'nf' 'let x : Nat = plus zero zero; let p : Id Nat x zero = refl; p' 'exact' (NF 'refl' 'Id Nat zero zero')
C 'C08' 'C-defeq' 'nf' ('let f : Nat -> Nat = ' + $L + ' x . plus x zero; let p : Id Nat (f zero) zero = refl; p') 'exact' (NF 'refl' 'Id Nat zero zero')
C 'C09' 'C-defeq' 'nf' ('let f : Nat -> Nat = ' + $L + ' x . plus x zero; let p : Id Nat (f (f zero)) zero = refl; p') 'exact' (NF 'refl' 'Id Nat zero zero')
C 'C10' 'C-defeq' 'nf' (RE 'zero' 'suc zero')                  'err' 'expecting zero equals to succ zero'
C 'C11' 'C-defeq' 'nf' (RE 'plus (suc zero) zero' 'zero')      'err' 'expecting succ zero equals to zero'
C 'C12' 'C-defeq' 'nf' (RE 'plus zero (suc zero)' 'zero')      'err' 'expecting succ zero equals to zero'
C 'C13' 'C-defeq' 'nf' ('let f : Nat -> Nat = ' + $L + ' x . plus x zero; let g : Nat -> Nat = ' + $L + ' x . x; let p : Id (Nat -> Nat) f g = refl; p') 'err' 'but it is not'
C 'C14' 'C-defeq' 'nf' ('let f : Nat -> Nat = ' + $L + ' x . plus zero x; let g : Nat -> Nat = ' + $L + ' x . x; let p : Id (Nat -> Nat) f g = refl; p') 'exact' (NF 'refl' ('Id (Nat ' + $AR + ' Nat) (' + $L + ' x . x) (' + $L + ' x . x)'))
C 'C15' 'C-defeq' 'nf' ('let f : Nat -> Nat = ' + $L + ' x . plus (suc zero) x; let g : Nat -> Nat = ' + $L + ' x . suc x; let p : Id (Nat -> Nat) f g = refl; p') 'exact' (NF 'refl' ('Id (Nat ' + $AR + ' Nat) (' + $L + ' x . succ x) (' + $L + ' x . succ x)'))
C 'C16' 'C-defeq' 'nf' ('let f : (A : U) -> A -> A = ' + $L + ' A x . x; let g : (A : U) -> A -> A = ' + $L + ' B y . y; let p : Id ((A : U) -> A -> A) f g = refl; p') 'exact' (NF 'refl' ('Id ((A : U) ' + $AR + ' A ' + $AR + ' A) (' + $L + ' A x . x) (' + $L + ' B y . y)'))
C 'C17' 'C-defeq' 'nf' ('let f : Nat -> Nat -> Nat = ' + $L + ' x y . x; let g : Nat -> Nat -> Nat = ' + $L + ' y x . x; let p : Id (Nat -> Nat -> Nat) f g = refl; p') 'err' 'but it is not'
C 'C18' 'C-defeq' 'nf' 'let p : Id (Id Nat zero zero) refl refl = refl; p' 'exact' (NF 'refl' 'Id (Id Nat zero zero) refl refl')
C 'C19' 'C-defeq' 'nf' 'let p : Id U Nat (Nat -> Nat) = refl; p' 'err' ('expecting Nat equals to Nat ' + $AR + ' Nat')
C 'C20' 'C-defeq' 'nf' 'let p : Id U Nat Nat = refl; p'        'exact' (NF 'refl' 'Id U Nat Nat')
C 'C21' 'C-defeq' 'nf' ('let g : Nat -> Nat = ' + $L + ' x . x; let eta : (g : Nat -> Nat) -> Id (Nat -> Nat) (' + $L + ' x . g x) g = ' + $L + ' g . refl; eta') 'info' ''
C 'C22' 'C-defeq' 'nf' ('let g : Nat -> Nat = ' + $L + ' x . x; let f : Nat -> Nat = ' + $L + ' x . g x; let p : Id (Nat -> Nat) f g = refl; p') 'exact' (NF 'refl' ('Id (Nat ' + $AR + ' Nat) (' + $L + ' x . x) (' + $L + ' x . x)'))
C 'C23' 'C-defeq' 'nf' (RE ('plus (' + (Num 6) + ') zero') (Num 6)) 'exact' (NF 'refl' ('Id Nat (' + (NumNF 6) + ') (' + (NumNF 6) + ')'))
C 'C24' 'C-defeq' 'nf' (RE ('plus zero (' + (Num 6) + ')') (Num 6)) 'exact' (NF 'refl' ('Id Nat (' + (NumNF 6) + ') (' + (NumNF 6) + ')'))
C 'C25' 'C-defeq' 'nf' (RE ('plus (' + (Num 3) + ') (' + (Num 3) + ')') (Num 6)) 'exact' (NF 'refl' ('Id Nat (' + (NumNF 6) + ') (' + (NumNF 6) + ')'))

# ---------------- D. errors and their positions
C 'D01' 'D-err' 'nf' 'foo'                                         'err' '^<stdin>:1:1:'
C 'D02' 'D-err' 'nf' '   foo'                                      'err' '<stdin>:1:4:'
C 'D03' 'D-err' 'nf' ('-- c1' + $NL + '-- c2' + $NL + 'bar')          'err' '<stdin>:3:1:'
C 'D04' 'D-err' 'nf' 'let bad : Nat = U; U'                        'err' '<stdin>:1:17:'
C 'D05' 'D-err' 'nf' ('let a : Nat = zero;' + $NL + 'let b : Nat = U;' + $NL + 'b') 'err' '<stdin>:2:15:'
C 'D06' 'D-err' 'nf' 'let p : Id Nat (suc zero) zero = refl; p'    'err' '<stdin>:1:34:'
C 'D07' 'D-err' 'nf' 'let a : Nat = zero; a a'                     'err' '<stdin>:1:21:'
C 'D08' 'D-err' 'nf' ('let f : Nat -> Nat = ' + $L + ' zero . zero; f') 'err' "'zero' is a keyword"
C 'D09' 'D-err' 'nf' ('let f : Nat -> Nat = ' + $L + ' suc . zero; f')  'err' "'suc' is a keyword"
C 'D10' 'D-err' 'nf' 'let Nat : U = Nat; Nat'                      'err' "'Nat' is a keyword"
C 'D11' 'D-err' 'nf' 'let refl : Nat = zero; refl'                 'err' "'refl' is a keyword"
C 'D12' 'D-err' 'nf' 'Id'                                          'err' '<stdin>:1:3:'
C 'D13' 'D-err' 'nf' 'suc'                                         'err' '<stdin>:1:4:'
C 'D14' 'D-err' 'nf' 'plus zero'                                   'err' '<stdin>:1:10:'
C 'D15' 'D-err' 'nf' 'suc U'                                       'err' '<stdin>:1:5:'
C 'D16' 'D-err' 'nf' 'plus U zero'                                 'err' '<stdin>:1:6:'
C 'D17' 'D-err' 'nf' 'Id zero zero zero'                           'err' '<stdin>:1:4:'
C 'D18' 'D-err' 'nf' ('let f : Nat -> Nat = ' + $L + ' x . x; f zero zero') 'err' 'Expected a function type'
C 'D19' 'D-err' 'nf' ($L + ' x . x')                               'err' "Can't infer type for lambda expression"
C 'D20' 'D-err' 'nf' 'refl'                                        'err' "Can't infer type for refl"
C 'D21' 'D-err' 'nf' ('let f : Id Nat (suc zero) zero -> Nat = ' + $L + ' x (!); f') 'err' 'Too many patterns'
C 'D22' 'D-err' 'nf' 'let x : Nat = zero'                          'err' "expected ';'"
C 'D23' 'D-err' 'nf' 'let x = zero; zero'                          'err' "expected ':'"
C 'D24' 'D-err' 'nf' 'U : U'                                       'err' 'expected end of input'
C 'D25' 'D-err' 'nf' '(Id)'                                        'err' '<stdin>:1:4:'
C 'D26' 'D-err' 'nf' '{- unterminated'                             'err' '<stdin>:1:16:'
C 'D27' 'D-err' 'nf' ''                                            'err' '<stdin>:1:1:'
C 'D28' 'D-err' 'nf' ('let f : (A : U) -> Id Nat zero zero -> A = ' + $L + ' A (!); f') 'err' "It should not be '\(\!\)'"
C 'D29' 'D-err' 'nf' ('let f : Nat -> Nat = ' + $L + ' refl . zero; f') 'err' 'Expected an identity type for pattern'
C 'D30' 'D-err' 'nf' ('let f : (n : Nat) -> Id Nat (suc n) zero -> Nat = ' + $L + ' n (!); f') 'ok' ''  # BUG 1 (panic)
C 'D31' 'D-err' 'nf' ('let f : Id Nat (suc zero) zero -> Nat = ' + $L + ' (!); f') 'info' ''  # BUG 2 (parse, spec-ambiguous)
C 'D32' 'D-err' 'nf' 'plus zero zero zero'                         'err' 'Expected a function type'
C 'D33' 'D-err' 'nf' 'Nat Nat'                                     'err' '<stdin>:1:1:'
C 'D34' 'D-err' 'nf' ('let f : Nat -> Nat = ' + $L + ' x x . x; f') 'err' 'Too many patterns'
C 'D35' 'D-err' 'nf' 'Id Nat zero zero -> Id Nat zero'             'err' '<stdin>:1:32:'

# ---------------- F. soundness: no false closed proof may be derived
C 'F01' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat (plus m zero) zero = ' + $L + ' m refl . refl; f') 'err' 'but it is not'
C 'F02' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat m zero = ' + $L + ' m refl . refl; f') 'err' 'but it is not'
C 'F03' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat (suc zero) zero = ' + $L + ' m refl . refl; f') 'err' 'but it is not'
C 'F04' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat zero (suc zero) = ' + $L + ' m refl . refl; f') 'err' 'but it is not'
C 'F05' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat (suc zero) zero = ' + $L + ' m refl . refl; f') 'err' 'but it is not'
C 'F06' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat m (suc (suc m)) = ' + $L + ' m refl . refl; f') 'err' 'but it is not'
C 'F07' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat m (suc m) -> Nat = ' + $L + ' m refl . zero; f zero refl') 'err' 'expecting zero equals to succ zero'
C 'F08' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat m (suc m) -> Nat = ' + $L + ' m refl . zero; f (suc zero) refl') 'err' 'but it is not'
C 'F09' 'F-sound' 'nf' (($C + 'let bad : Nat = C Nat; bad')) 'err' 'type mismatch'
C 'F10' 'F-sound' 'nf' (($C + 'let bad : Id Nat zero (suc zero) = C (Id Nat zero (suc zero)); bad')) 'err' 'type mismatch'
C 'F11' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat (suc m) zero -> Nat = ' + $L + ' m refl . zero; f') 'err' "'refl' for equation between succ m and zero"
C 'F12' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat (suc m) (suc (suc m)) -> Nat = ' + $L + ' m (!); f') 'err' "'\(\!\)' for equation between succ m and succ \(succ m\)"
# accepted although the equation is not decidable by first-order unification (documented with-K design)
C 'F13' 'F-sound' 'nf' ('let f : (m : Nat) -> Id Nat (plus m zero) m -> Nat = ' + $L + ' m refl . zero; f') 'ok' ''

# ---------------- G. pattern grammar edges noted in the report
C 'G01' 'G-grammar' 'nf' ('let f : Id Nat (suc zero) zero -> Nat = ' + $L + ' (!); f') 'info' ''
C 'G02' 'G-grammar' 'nf' ('let f : Nat -> Id Nat (suc zero) zero -> Nat = ' + $L + ' (!) x; f') 'info' ''
C 'G03' 'G-grammar' 'nf' ('let f : Nat -> Nat = ' + $L + ' _ . zero; f') 'info' ''
C 'G04' 'G-grammar' 'nf' ('let f : Nat -> Nat -> Nat = ' + $L + ' x x . x; f (suc zero) (suc (suc zero))') 'exact' (NF 'succ (succ zero)' 'Nat')
C 'G05' 'G-grammar' 'nf' 'let _ : Nat = zero; zero'             'exact' (NF 'zero' 'Nat')

# ---------------- run A-D
$results = @()
$invocations = 0
foreach ($c in $cases) {
  $r = Run-Case -Id $c.i -Mode $c.m -Text $c.t -TimeoutSec 15 -Tag $c.tag
  $invocations++
  if ($c.k -eq 'info') { $results += (Assert-Case -R $r -Kind 'any') }
  else { $results += (Assert-Case -R $r -Kind $c.k -Expect $c.e) }
}
Write-Host ("phase A-D: {0} cases, {1} invocations" -f $cases.Count, $invocations)

# ============================================================ E. laws
function Split-Nf([string]$out) {
  $lines = $out -split "`n"
  # find the '  :' separator line
  $idx = -1
  for ($k=0; $k -lt $lines.Count; $k++) { if ($lines[$k] -ceq '  :') { $idx = $k; break } }
  if ($idx -lt 0) { return $null }
  $term = ($lines[0..($idx-1)] -join "`n")
  $ty   = ($lines[($idx+1)..($lines.Count-1)] -join "`n").TrimEnd("`n")
  return @{ term = $term; ty = $ty }
}
function Judge([string]$id,[string]$tag,[string]$mode,[string]$text,$r,[bool]$ok,[string]$why) {
  $script:invocations++
  $v = 'FAIL'; $w = $why
  if ($ok) { $v = 'PASS'; $w = '' }
  $rec = [PSCustomObject]@{
    Id=$id; Tag=$tag; Mode=$mode; Ms=$r.Ms; Exit=$r.Exit; StdOut=$r.StdOut; StdErr=$r.StdErr
    StdIn=$text; Kind='law'; Expect=''; Verdict=$v; Why=$w
  }
  return $rec
}

# E1/E2/E3: round-trip and agreement laws over a set of terms
$rtTerms = @(
  @{id='E01'; t='plus (suc zero) (suc zero)'},
  @{id='E02'; t='Id Nat zero zero -> Nat'},
  @{id='E03'; t='U'},
  @{id='E04'; t=('let f : Nat -> Nat = ' + $L + ' x . suc x; f')},
  @{id='E05'; t=('let f : (A : U) -> A -> A = ' + $L + ' A x . x; f')},
  @{id='E06'; t=($H + 'H zero')},
  @{id='E07'; t='(A : U) -> (A : U) -> A'},
  @{id='E08'; t=($TRANS + 'trans Nat zero zero zero refl refl')}
)
foreach ($rt in $rtTerms) {
  $r = Run-Case -Id $rt.id -Mode 'nf' -Text $rt.t -TimeoutSec 15 -Tag 'E-law-roundtrip'
  $split = Split-Nf $r.StdOut
  if ($r.Exit -ne 0 -or $null -eq $split) {
    $results += (Judge $rt.id 'E-law-roundtrip' 'nf' $rt.t $r $false ('nf failed or unparseable output: exit=' + $r.Exit)); continue
  }
  $origTerm = $split.term; $origTy = $split.ty
  # E-x-a: the (term,type) pair printed by nf must itself re-typecheck, unchanged
  $feedRaw = 'let z : ' + $origTy + ' = ' + $origTerm + '; z'
  $r2 = Run-Case -Id ($rt.id + 'a') -Mode 'nf' -Text $feedRaw -TimeoutSec 15 -Tag 'E-law-roundtrip'
  $ok2 = ($r2.Exit -eq 0) -and ($r2.StdOut -ceq $r.StdOut)
  $results += (Judge ($rt.id + 'a') 'E-law-roundtrip' 'nf' $feedRaw $r2 $ok2 ('re-checking the printed normal form at its printed type failed or changed it: exit=' + $r2.Exit + ' stderr=' + (($r2.StdErr -replace "`r?`n",' | ')) + ' out=' + (($r2.StdOut -replace "`r?`n",' | '))))
  # E-x-b: same, with the printer's 'succ' mapped back to the parser's 'suc' (isolates the succ defect)
  $feedFix = 'let z : ' + (Jet $origTy) + ' = ' + (Jet $origTerm) + '; z'
  $r3 = Run-Case -Id ($rt.id + 'b') -Mode 'nf' -Text $feedFix -TimeoutSec 15 -Tag 'E-law-roundtrip'
  $ok3 = ($r3.Exit -eq 0) -and ((Jet $r3.StdOut) -ceq (Jet $r.StdOut))
  $results += (Judge ($rt.id + 'b') 'E-law-roundtrip' 'nf' $feedFix $r3 $ok3 ('still fails after succ->suc mapping: exit=' + $r3.Exit + ' stderr=' + (($r3.StdErr -replace "`r?`n",' | ')) + ' out=' + (($r3.StdOut -replace "`r?`n",' | '))))
  # E-x-c: type mode must agree with the type part of nf mode
  $r4 = Run-Case -Id ($rt.id + 'c') -Mode 'type' -Text $rt.t -TimeoutSec 15 -Tag 'E-law-agree'
  $tyOut = ($r4.StdOut -replace "`r?`n", "`n").TrimEnd("`n")
  $results += (Judge ($rt.id + 'c') 'E-law-agree' 'type' $rt.t $r4 (($r4.Exit -eq 0) -and ($tyOut -ceq $origTy)) ('type mode gave <' + $tyOut + '> but nf mode printed type <' + $origTy + '>'))
}

# E5: plus is addition on 0..6
$plusFails = @()
foreach ($m in 0..6) { foreach ($n in 0..6) {
  $t = 'plus (' + (Num $m) + ') (' + (Num $n) + ')'
  $r = Run-Case -Id ('E5-' + $m + '-' + $n) -Mode 'nf' -Text $t -TimeoutSec 15 -Tag 'E-law-plus'
  $exp = NF (NumNF ($m+$n)) 'Nat'
  $ok = ($r.Exit -eq 0) -and ($r.StdOut -ceq $exp)
  $results += (Judge ('E5-' + $m + '-' + $n) 'E-law-plus' 'nf' $t $r $ok ('expected ' + ($exp -replace "`r?`n",' | ') + ' got ' + ($r.StdOut -replace "`r?`n",' | ')))
  if (-not $ok) { $plusFails += "$m+$n" }
} }

# E6: stability under let-inlining
$inlinePairs = @(
  @{id='E06'; a='let x : Nat = plus zero zero; suc x';                     b='suc (plus zero zero)'},
  @{id='E07'; a='let x : Nat = zero; plus x (suc zero)';                   b='plus zero (suc zero)'},
  @{id='E08'; a='let x : Nat = suc zero; let y : Nat = plus x x; y';       b='plus (suc zero) (suc zero)'},
  @{id='E09'; a='let A : U = Nat; A';                                      b='Nat'},
  @{id='E10'; a='let x : Id Nat zero zero = refl; x';                      b='let y : Id Nat zero zero = refl; y'},
  @{id='E11'; a=('let f : Nat -> Nat = ' + $L + ' x . suc x; f (suc zero)');   b='suc (suc zero)'}
)
foreach ($p in $inlinePairs) {
  $ra = Run-Case -Id ($p.id + 'L') -Mode 'nf' -Text $p.a -TimeoutSec 15 -Tag 'E-law-inline'
  $rb = Run-Case -Id ($p.id + 'R') -Mode 'nf' -Text $p.b -TimeoutSec 15 -Tag 'E-law-inline'
  $ok = ($ra.Exit -eq 0) -and ($rb.Exit -eq 0) -and ($ra.StdOut -ceq $rb.StdOut)
  $results += (Judge ($p.id + 'L') 'E-law-inline' 'nf' $p.a $ra $ok ('let-form <' + ($ra.StdOut -replace "`r?`n",' | ') + '> vs inlined <' + ($rb.StdOut -replace "`r?`n",' | ') + '>'))
}

# E12: determinism (same input twice, byte-identical)
foreach ($t in @('plus (suc zero) (suc zero)', ($H + 'H zero zero refl refl'), 'Id Nat zero zero -> Nat')) {
  $x = Run-Case -Id 'E12x' -Mode 'nf' -Text $t -TimeoutSec 15 -Tag 'E-law-det'
  $y = Run-Case -Id 'E12y' -Mode 'nf' -Text $t -TimeoutSec 15 -Tag 'E-law-det'
  $results += (Judge 'E12' 'E-law-det' 'nf' $t $y (($x.StdOut -ceq $y.StdOut) -and ($x.Exit -eq $y.Exit)) 'two identical runs differed')
}

# E13: the type printed by nf must itself be a well-formed term already in normal form
foreach ($t in @(($H + 'H zero'), ('let p : Id Nat (plus zero zero) zero = refl; p'), ('(A : U) -> (A : U) -> A'))) {
  $r = Run-Case -Id 'E13x' -Mode 'nf' -Text $t -TimeoutSec 15 -Tag 'E-law-type-rt'
  $sp = Split-Nf $r.StdOut
  if ($null -eq $sp) { $results += (Judge 'E13' 'E-law-type-rt' 'nf' $t $r $false 'no nf output'); continue }
  $r2 = Run-Case -Id 'E13y' -Mode 'nf' -Text $sp.ty -TimeoutSec 15 -Tag 'E-law-type-rt'
  $sp2 = Split-Nf $r2.StdOut
  $ok = ($r2.Exit -eq 0) -and ($null -ne $sp2) -and ((Jet $sp2.term) -ceq (Jet $sp.ty))
  $results += (Judge 'E13' 'E-law-type-rt' 'nf' $sp.ty $r2 $ok ('printed type is not a re-parseable normal form: exit=' + $r2.Exit + ' stderr=' + (($r2.StdErr -replace "`r?`n",' | ')) + ' out=' + (($r2.StdOut -replace "`r?`n",' | '))))
}

# ---------------- summary + artifacts
$all = $results
$log = New-Object System.Text.StringBuilder
foreach ($r in $all) { [void]$log.Append((Render-Record $r)) }
Write-Text (Join-Path $script:WORK 'all.log') $log.ToString()
$csv = $all | Select-Object Id,Tag,Mode,Exit,Verdict,Ms,Why | ConvertTo-Csv -NoTypeInformation
Write-Text (Join-Path $script:WORK 'all.csv') ($csv -join "`r`n")

$pass = ($all | Where-Object { $_.Verdict -eq 'PASS' }).Count
$fail = ($all | Where-Object { $_.Verdict -eq 'FAIL' }).Count
$info = ($all | Where-Object { $_.Verdict -eq 'INFO' }).Count
Write-Host ""
Write-Host "================ SUMMARY ================"
Write-Host ("invocations: {0}   records: {1}" -f $invocations, $all.Count)
Write-Host ("PASS: {0}   FAIL: {1}   INFO(no verdict): {2}" -f $pass, $fail, $info)
Write-Host ""
Write-Host "--- FAILURES ---"
$all | Where-Object { $_.Verdict -eq 'FAIL' } | ForEach-Object {
  Write-Host ("[{0}] {1} mode={2} exit={3}" -f $_.Id, $_.Tag, $_.Mode, $_.Exit)
  Write-Host ("   stdin : " + (($_.StdIn -replace "`r?`n",' \n ') -replace '\s+$',''))
  Write-Host ("   stdout: " + (($_.StdOut -replace "`r?`n",' | ')))
  Write-Host ("   stderr: " + (($_.StdErr -replace "`r?`n",' | ')))
  Write-Host ("   why   : " + (($_.Why -replace "`r?`n",' | ')))
  Write-Host ""
}
Write-Host "--- INFO (no expectation asserted) ---"
$all | Where-Object { $_.Verdict -eq 'INFO' } | ForEach-Object {
  Write-Host ("[{0}] exit={1} stdin=<{2}> out=<{3}> err=<{4}>" -f $_.Id, $_.Exit, (($_.StdIn -replace "`r?`n",' \n ')), (($_.StdOut -replace "`r?`n",' | ')), (($_.StdErr -replace "`r?`n",' | ')))
}
Write-Host ""
Write-Host "--- per tag ---"
$all | Group-Object Tag | Sort-Object Name | ForEach-Object {
  $p = @($_.Group | Where-Object {$_.Verdict -eq 'PASS'}).Count
  $f = @($_.Group | Where-Object {$_.Verdict -eq 'FAIL'}).Count
  $i = @($_.Group | Where-Object {$_.Verdict -eq 'INFO'}).Count
  Write-Host ("{0,-22} n={1,-4} pass={2,-4} fail={3,-4} info={4}" -f $_.Name, $_.Count, $p, $f, $i)
}
Write-Host ""
Write-Host "EXIT CODES SEEN:"
$all | Group-Object Exit | Sort-Object Name | ForEach-Object { Write-Host ("  exit={0} : {1}" -f $_.Name, $_.Count) }
