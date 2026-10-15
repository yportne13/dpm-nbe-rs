# dpm-nbe black-box test results

Binary: `F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe`  
Runner: `run-all.ps1` (built on `lib.ps1`), byte tests: `bytes-tests.ps1`  
Full raw log: `command-log.txt`; machine-readable: `test-table.csv`

**Totals: 230 CLI invocations / records - PASS 214, FAIL 10, no-verdict(INFO) 6.**

| id | tag | mode | exit | verdict | input (truncated to 90 chars) |
| --- | --- | --- | --- | --- | --- |
| A01 | A-basic | nf | 0 | PASS | `U` |
| A02 | A-basic | nf | 0 | PASS | `Nat` |
| A03 | A-basic | type | 0 | PASS | `Nat` |
| A04 | A-basic | nf | 0 | PASS | `zero` |
| A05 | A-basic | type | 0 | PASS | `zero` |
| A06 | A-basic | nf | 0 | PASS | `suc zero` |
| A07 | A-basic | nf | 0 | PASS | `suc (suc (suc zero))` |
| A08 | A-basic | nf | 0 | PASS | `plus zero zero` |
| A09 | A-basic | nf | 0 | PASS | `plus zero (suc zero)` |
| A10 | A-basic | nf | 0 | PASS | `plus (suc zero) zero` |
| A11 | A-basic | nf | 0 | PASS | `plus (suc (suc (zero))) (suc (suc (suc (zero))))` |
| A12 | A-basic | nf | 0 | PASS | `Id Nat zero zero` |
| A13 | A-basic | type | 0 | PASS | `Id Nat zero zero` |
| A14 | A-basic | nf | 0 | PASS | `(A : U) -> A -> A` |
| A15 | A-basic | type | 0 | PASS | `(A : U) -> A -> A` |
| A16 | A-basic | nf | 0 | PASS | `Nat -> Nat` |
| A17 | A-basic | nf | 0 | PASS | `(A : U) → A → A` |
| A18 | A-basic | nf | 0 | PASS | `-- comment \n  zero` |
| A19 | A-basic | nf | 0 | PASS | `{- comment -} zero` |
| A20 | A-basic | nf | 0 | PASS | `{- a -} {- b -} zero` |
| A21 | A-basic | nf | 0 | PASS | `zero -- trailing` |
| A22 | A-basic | nf | 0 | PASS | `let x : Nat = zero; x` |
| A23 | A-basic | nf | 0 | PASS | `let x : Nat = zero; let y : Nat = suc x; y` |
| A24 | A-basic | nf | 0 | PASS | `let x : Nat = zero; let x : Nat = suc x; x` |
| A25 | A-basic | nf | 0 | PASS | `let f : Nat -> Nat = λ x . suc x; f zero` |
| A26 | A-basic | nf | 0 | PASS | `let f : (A : U) -> A -> A = λ A x . x; f Nat zero` |
| A27 | A-basic | nf | 0 | PASS | `let f : (A B : U)(g : A -> B)(x : A) -> B = λ A B g x . g x; f Nat Nat (λ n . suc n) zero` |
| A28 | A-basic | type | 0 | PASS | `let f : Nat -> Nat = λ x . suc x; f` |
| A29 | A-basic | nf | 0 | PASS | `Id U Nat Nat` |
| A30 | A-basic | nf | 0 | PASS | `(x : Nat) -> Id Nat x x` |
| A31 | A-basic | nf | 0 | PASS | `(Nat -> Nat) -> Nat` |
| A32 | A-basic | nf | 0 | PASS | `(A : U) -> (A : U) -> A` |
| A33 | A-basic | nf | 0 | PASS | `let f : Nat -> Nat -> Nat = λ x x . x; f (suc zero) (suc (suc zero))` |
| A34 | A-basic | nf | 0 | PASS | `let f : Nat -> Nat = λ x . x; let g : Nat -> Nat = λ x . x; g zero` |
| A35 | A-basic | nf | 0 | PASS | `let _ : Nat = zero; zero` |
| A36 | A-basic | nf | 1 | PASS | `let _ : Nat = zero; _` |
| A37 | A-basic | nf | 1 | INFO | `{- a {- b -} -} zero` |
| B01 | B-readme | type | 0 | PASS | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = \n   λ A a b c refl ...` |
| B02 | B-readme | nf | 0 | PASS | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = \n   λ A a b c refl ...` |
| B03 | B-readme | type | 0 | PASS | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = \n   λ A a b c refl ...` |
| B04 | B-readme | nf | 0 | PASS | `let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = \n   λ A a b c refl ...` |
| B05 | B-readme | type | 0 | PASS | `let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl ....` |
| B06 | B-readme | nf | 0 | PASS | `let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl ....` |
| B07 | B-readme | type | 0 | PASS | `let cong : (A B : U)(f : A -> B)(a b : A) -> Id A a b -> Id B (f a) (f b) = λ A B f a b re...` |
| B08 | B-readme | nf | 0 | PASS | `let cong : (A B : U)(f : A -> B)(a b : A) -> Id A a b -> Id B (f a) (f b) = λ A B f a b re...` |
| B09 | B-readme | type | 0 | PASS | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n re...` |
| B10 | B-readme | nf | 0 | PASS | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n re...` |
| B11 | B-readme | type | 0 | PASS | `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); U` |
| B12 | B-readme | nf | 1 | PASS | `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C Nat refl` |
| B13 | B-readme | nf | 101 | FAIL | `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C Nat` |
| B14 | B-readme | type | 0 | PASS | `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C Nat` |
| B15 | B-gen | type | 0 | PASS | `let trans3 : (A : U)(a b c d : A) -> Id A a b -> Id A b c -> Id A c d -> Id A a d = λ A a ...` |
| B16 | B-gen | nf | 0 | PASS | `let trans3 : (A : U)(a b c d : A) -> Id A a b -> Id A b c -> Id A c d -> Id A a d = λ A a ...` |
| B17 | B-gen | type | 0 | PASS | `let trans4 : (A : U)(a b c d e : A) -> Id A a b -> Id A b c -> Id A c d -> Id A d e -> Id ...` |
| B18 | B-gen | nf | 0 | PASS | `let trans4 : (A : U)(a b c d e : A) -> Id A a b -> Id A b c -> Id A c d -> Id A d e -> Id ...` |
| B19 | B-gen | type | 0 | PASS | `let trans5 : (A : U)(a b c d e f : A) -> Id A a b -> Id A b c -> Id A c d -> Id A d e -> I...` |
| B20 | B-gen | nf | 0 | PASS | `let trans5 : (A : U)(a b c d e f : A) -> Id A a b -> Id A b c -> Id A c d -> Id A d e -> I...` |
| B21 | B-gen | nf | 0 | PASS | `let trans2 : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl ...` |
| B22 | B-gen | nf | 0 | PASS | `let trans2 : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl ...` |
| B23 | B-gen | nf | 0 | PASS | `let trans2 : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl ...` |
| B24 | B-gen | nf | 0 | PASS | `let trans2 : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl ...` |
| B25 | B-gen | nf | 1 | PASS | `let trans2 : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl ...` |
| B26 | B-gen | nf | 1 | PASS | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n re...` |
| B27 | B-gen | nf | 1 | PASS | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n re...` |
| B28 | B-gen | nf | 1 | PASS | `let cong : (A B : U)(f : A -> B)(a b : A) -> Id A a b -> Id B (f a) (f b) = λ A B f a b re...` |
| B29 | B-gen | nf | 0 | FAIL | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n re...` |
| B30 | B-gen | nf | 0 | FAIL | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n re...` |
| C01 | C-defeq | nf | 0 | PASS | `let p : Id Nat (plus zero zero) (zero) = refl; p` |
| C02 | C-defeq | nf | 0 | PASS | `let p : Id Nat (plus (suc zero) zero) (suc zero) = refl; p` |
| C03 | C-defeq | nf | 0 | PASS | `let p : Id Nat (plus zero (suc zero)) (suc zero) = refl; p` |
| C04 | C-defeq | nf | 0 | PASS | `let p : Id Nat (plus (suc (suc (zero))) (suc (suc (zero)))) (suc (suc (suc (suc (zero)))))...` |
| C05 | C-defeq | nf | 0 | PASS | `let p : Id Nat (zero) (plus zero zero) = refl; p` |
| C06 | C-defeq | nf | 0 | PASS | `let p : Id Nat (plus (plus zero zero) zero) (zero) = refl; p` |
| C07 | C-defeq | nf | 0 | PASS | `let x : Nat = plus zero zero; let p : Id Nat x zero = refl; p` |
| C08 | C-defeq | nf | 0 | PASS | `let f : Nat -> Nat = λ x . plus x zero; let p : Id Nat (f zero) zero = refl; p` |
| C09 | C-defeq | nf | 0 | PASS | `let f : Nat -> Nat = λ x . plus x zero; let p : Id Nat (f (f zero)) zero = refl; p` |
| C10 | C-defeq | nf | 1 | PASS | `let p : Id Nat (zero) (suc zero) = refl; p` |
| C11 | C-defeq | nf | 1 | PASS | `let p : Id Nat (plus (suc zero) zero) (zero) = refl; p` |
| C12 | C-defeq | nf | 1 | PASS | `let p : Id Nat (plus zero (suc zero)) (zero) = refl; p` |
| C13 | C-defeq | nf | 1 | PASS | `let f : Nat -> Nat = λ x . plus x zero; let g : Nat -> Nat = λ x . x; let p : Id (Nat -> N...` |
| C14 | C-defeq | nf | 0 | PASS | `let f : Nat -> Nat = λ x . plus zero x; let g : Nat -> Nat = λ x . x; let p : Id (Nat -> N...` |
| C15 | C-defeq | nf | 0 | PASS | `let f : Nat -> Nat = λ x . plus (suc zero) x; let g : Nat -> Nat = λ x . suc x; let p : Id...` |
| C16 | C-defeq | nf | 0 | PASS | `let f : (A : U) -> A -> A = λ A x . x; let g : (A : U) -> A -> A = λ B y . y; let p : Id (...` |
| C17 | C-defeq | nf | 1 | PASS | `let f : Nat -> Nat -> Nat = λ x y . x; let g : Nat -> Nat -> Nat = λ y x . x; let p : Id (...` |
| C18 | C-defeq | nf | 0 | PASS | `let p : Id (Id Nat zero zero) refl refl = refl; p` |
| C19 | C-defeq | nf | 1 | PASS | `let p : Id U Nat (Nat -> Nat) = refl; p` |
| C20 | C-defeq | nf | 0 | PASS | `let p : Id U Nat Nat = refl; p` |
| C21 | C-defeq | nf | 1 | INFO | `let g : Nat -> Nat = λ x . x; let eta : (g : Nat -> Nat) -> Id (Nat -> Nat) (λ x . g x) g ...` |
| C22 | C-defeq | nf | 0 | PASS | `let g : Nat -> Nat = λ x . x; let f : Nat -> Nat = λ x . g x; let p : Id (Nat -> Nat) f g ...` |
| C23 | C-defeq | nf | 0 | PASS | `let p : Id Nat (plus (suc (suc (suc (suc (suc (suc (zero))))))) zero) (suc (suc (suc (suc ...` |
| C24 | C-defeq | nf | 0 | PASS | `let p : Id Nat (plus zero (suc (suc (suc (suc (suc (suc (zero)))))))) (suc (suc (suc (suc ...` |
| C25 | C-defeq | nf | 0 | PASS | `let p : Id Nat (plus (suc (suc (suc (zero)))) (suc (suc (suc (zero))))) (suc (suc (suc (su...` |
| D01 | D-err | nf | 1 | PASS | `foo` |
| D02 | D-err | nf | 1 | PASS | `   foo` |
| D03 | D-err | nf | 1 | PASS | `-- c1 \n -- c2 \n bar` |
| D04 | D-err | nf | 1 | PASS | `let bad : Nat = U; U` |
| D05 | D-err | nf | 1 | PASS | `let a : Nat = zero; \n let b : Nat = U; \n b` |
| D06 | D-err | nf | 1 | PASS | `let p : Id Nat (suc zero) zero = refl; p` |
| D07 | D-err | nf | 1 | PASS | `let a : Nat = zero; a a` |
| D08 | D-err | nf | 1 | PASS | `let f : Nat -> Nat = λ zero . zero; f` |
| D09 | D-err | nf | 1 | PASS | `let f : Nat -> Nat = λ suc . zero; f` |
| D10 | D-err | nf | 1 | PASS | `let Nat : U = Nat; Nat` |
| D11 | D-err | nf | 1 | PASS | `let refl : Nat = zero; refl` |
| D12 | D-err | nf | 1 | PASS | `Id` |
| D13 | D-err | nf | 1 | PASS | `suc` |
| D14 | D-err | nf | 1 | PASS | `plus zero` |
| D15 | D-err | nf | 1 | PASS | `suc U` |
| D16 | D-err | nf | 1 | PASS | `plus U zero` |
| D17 | D-err | nf | 1 | PASS | `Id zero zero zero` |
| D18 | D-err | nf | 1 | PASS | `let f : Nat -> Nat = λ x . x; f zero zero` |
| D19 | D-err | nf | 1 | PASS | `λ x . x` |
| D20 | D-err | nf | 1 | PASS | `refl` |
| D21 | D-err | nf | 1 | PASS | `let f : Id Nat (suc zero) zero -> Nat = λ x (!); f` |
| D22 | D-err | nf | 1 | PASS | `let x : Nat = zero` |
| D23 | D-err | nf | 1 | PASS | `let x = zero; zero` |
| D24 | D-err | nf | 1 | PASS | `U : U` |
| D25 | D-err | nf | 1 | PASS | `(Id)` |
| D26 | D-err | nf | 1 | PASS | `{- unterminated` |
| D27 | D-err | nf | 1 | PASS | `` |
| D28 | D-err | nf | 1 | PASS | `let f : (A : U) -> Id Nat zero zero -> A = λ A (!); f` |
| D29 | D-err | nf | 1 | PASS | `let f : Nat -> Nat = λ refl . zero; f` |
| D30 | D-err | nf | 101 | FAIL | `let f : (n : Nat) -> Id Nat (suc n) zero -> Nat = λ n (!); f` |
| D31 | D-err | nf | 1 | INFO | `let f : Id Nat (suc zero) zero -> Nat = λ (!); f` |
| D32 | D-err | nf | 1 | PASS | `plus zero zero zero` |
| D33 | D-err | nf | 1 | PASS | `Nat Nat` |
| D34 | D-err | nf | 1 | PASS | `let f : Nat -> Nat = λ x x . x; f` |
| D35 | D-err | nf | 1 | PASS | `Id Nat zero zero -> Id Nat zero` |
| F01 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat (plus m zero) zero = λ m refl . refl...` |
| F02 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat m zero = λ m refl . refl; f` |
| F03 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat (plus m zero) m -> Id Nat (suc zero) zero = λ m refl . refl; f` |
| F04 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat zero (suc zero) = λ m refl . refl; f` |
| F05 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat (suc zero) zero = λ m refl . refl; f` |
| F06 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat m (suc m) -> Id Nat m (suc (suc m)) = λ m refl . refl; f` |
| F07 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat m (suc m) -> Nat = λ m refl . zero; f zero refl` |
| F08 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat m (suc m) -> Nat = λ m refl . zero; f (suc zero) refl` |
| F09 | F-sound | nf | 1 | PASS | `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); let bad : Nat = C Nat; bad` |
| F10 | F-sound | nf | 1 | PASS | `let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); let bad : Id Nat zero (suc zero)...` |
| F11 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat (suc m) zero -> Nat = λ m refl . zero; f` |
| F12 | F-sound | nf | 1 | PASS | `let f : (m : Nat) -> Id Nat (suc m) (suc (suc m)) -> Nat = λ m (!); f` |
| F13 | F-sound | nf | 0 | PASS | `let f : (m : Nat) -> Id Nat (plus m zero) m -> Nat = λ m refl . zero; f` |
| G01 | G-grammar | nf | 1 | INFO | `let f : Id Nat (suc zero) zero -> Nat = λ (!); f` |
| G02 | G-grammar | nf | 1 | INFO | `let f : Nat -> Id Nat (suc zero) zero -> Nat = λ (!) x; f` |
| G03 | G-grammar | nf | 1 | INFO | `let f : Nat -> Nat = λ _ . zero; f` |
| G04 | G-grammar | nf | 0 | PASS | `let f : Nat -> Nat -> Nat = λ x x . x; f (suc zero) (suc (suc zero))` |
| G05 | G-grammar | nf | 0 | PASS | `let _ : Nat = zero; zero` |
| E01a | E-law-roundtrip | nf | 1 | FAIL | `let z : Nat = succ (succ zero); z` |
| E01b | E-law-roundtrip | nf | 0 | PASS | `let z : Nat = suc (suc zero); z` |
| E01c | E-law-agree | type | 0 | PASS | `plus (suc zero) (suc zero)` |
| E02a | E-law-roundtrip | nf | 0 | PASS | `let z : U = Id Nat zero zero → Nat; z` |
| E02b | E-law-roundtrip | nf | 0 | PASS | `let z : U = Id Nat zero zero → Nat; z` |
| E02c | E-law-agree | type | 0 | PASS | `Id Nat zero zero -> Nat` |
| E03a | E-law-roundtrip | nf | 0 | PASS | `let z : U = U; z` |
| E03b | E-law-roundtrip | nf | 0 | PASS | `let z : U = U; z` |
| E03c | E-law-agree | type | 0 | PASS | `U` |
| E04a | E-law-roundtrip | nf | 1 | FAIL | `let z : Nat → Nat = λ x . succ x; z` |
| E04b | E-law-roundtrip | nf | 0 | PASS | `let z : Nat → Nat = λ x . suc x; z` |
| E04c | E-law-agree | type | 0 | PASS | `let f : Nat -> Nat = λ x . suc x; f` |
| E05a | E-law-roundtrip | nf | 0 | PASS | `let z : (A : U) → A → A = λ A x . x; z` |
| E05b | E-law-roundtrip | nf | 0 | PASS | `let z : (A : U) → A → A = λ A x . x; z` |
| E05c | E-law-agree | type | 0 | PASS | `let f : (A : U) -> A -> A = λ A x . x; f` |
| E06a | E-law-roundtrip | nf | 1 | FAIL | `let z : (n : Nat) → Id Nat n zero → Id Nat zero zero → Id Nat n zero = λ m n refl refl . r...` |
| E06b | E-law-roundtrip | nf | 1 | FAIL | `let z : (n : Nat) → Id Nat n zero → Id Nat zero zero → Id Nat n zero = λ m n refl refl . r...` |
| E06c | E-law-agree | type | 0 | PASS | `let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n re...` |
| E07a | E-law-roundtrip | nf | 1 | FAIL | `let z : U = (A : U)(A' : U) → A'; z` |
| E07b | E-law-roundtrip | nf | 1 | FAIL | `let z : U = (A : U)(A' : U) → A'; z` |
| E07c | E-law-agree | type | 0 | PASS | `(A : U) -> (A : U) -> A` |
| E08a | E-law-roundtrip | nf | 0 | PASS | `let z : Id Nat zero zero = refl; z` |
| E08b | E-law-roundtrip | nf | 0 | PASS | `let z : Id Nat zero zero = refl; z` |
| E08c | E-law-agree | type | 0 | PASS | `let trans : (A : U)(a b c : A) -> Id A a b -> Id A b c -> Id A a c = λ A a b c refl refl ....` |
| E5-0-0 | E-law-plus | nf | 0 | PASS | `plus (zero) (zero)` |
| E5-0-1 | E-law-plus | nf | 0 | PASS | `plus (zero) (suc (zero))` |
| E5-0-2 | E-law-plus | nf | 0 | PASS | `plus (zero) (suc (suc (zero)))` |
| E5-0-3 | E-law-plus | nf | 0 | PASS | `plus (zero) (suc (suc (suc (zero))))` |
| E5-0-4 | E-law-plus | nf | 0 | PASS | `plus (zero) (suc (suc (suc (suc (zero)))))` |
| E5-0-5 | E-law-plus | nf | 0 | PASS | `plus (zero) (suc (suc (suc (suc (suc (zero))))))` |
| E5-0-6 | E-law-plus | nf | 0 | PASS | `plus (zero) (suc (suc (suc (suc (suc (suc (zero)))))))` |
| E5-1-0 | E-law-plus | nf | 0 | PASS | `plus (suc (zero)) (zero)` |
| E5-1-1 | E-law-plus | nf | 0 | PASS | `plus (suc (zero)) (suc (zero))` |
| E5-1-2 | E-law-plus | nf | 0 | PASS | `plus (suc (zero)) (suc (suc (zero)))` |
| E5-1-3 | E-law-plus | nf | 0 | PASS | `plus (suc (zero)) (suc (suc (suc (zero))))` |
| E5-1-4 | E-law-plus | nf | 0 | PASS | `plus (suc (zero)) (suc (suc (suc (suc (zero)))))` |
| E5-1-5 | E-law-plus | nf | 0 | PASS | `plus (suc (zero)) (suc (suc (suc (suc (suc (zero))))))` |
| E5-1-6 | E-law-plus | nf | 0 | PASS | `plus (suc (zero)) (suc (suc (suc (suc (suc (suc (zero)))))))` |
| E5-2-0 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (zero))) (zero)` |
| E5-2-1 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (zero))) (suc (zero))` |
| E5-2-2 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (zero))) (suc (suc (zero)))` |
| E5-2-3 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (zero))) (suc (suc (suc (zero))))` |
| E5-2-4 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (zero))) (suc (suc (suc (suc (zero)))))` |
| E5-2-5 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (zero))) (suc (suc (suc (suc (suc (zero))))))` |
| E5-2-6 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (zero))) (suc (suc (suc (suc (suc (suc (zero)))))))` |
| E5-3-0 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (zero)))) (zero)` |
| E5-3-1 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (zero)))) (suc (zero))` |
| E5-3-2 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (zero)))) (suc (suc (zero)))` |
| E5-3-3 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (zero)))) (suc (suc (suc (zero))))` |
| E5-3-4 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (zero)))) (suc (suc (suc (suc (zero)))))` |
| E5-3-5 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (zero)))) (suc (suc (suc (suc (suc (zero))))))` |
| E5-3-6 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (zero)))) (suc (suc (suc (suc (suc (suc (zero)))))))` |
| E5-4-0 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (zero))))) (zero)` |
| E5-4-1 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (zero))))) (suc (zero))` |
| E5-4-2 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (zero))))) (suc (suc (zero)))` |
| E5-4-3 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (zero))))) (suc (suc (suc (zero))))` |
| E5-4-4 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (zero))))) (suc (suc (suc (suc (zero)))))` |
| E5-4-5 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (zero))))) (suc (suc (suc (suc (suc (zero))))))` |
| E5-4-6 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (zero))))) (suc (suc (suc (suc (suc (suc (zero)))))))` |
| E5-5-0 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (zero)))))) (zero)` |
| E5-5-1 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (zero)))))) (suc (zero))` |
| E5-5-2 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (zero)))))) (suc (suc (zero)))` |
| E5-5-3 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (zero)))))) (suc (suc (suc (zero))))` |
| E5-5-4 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (zero)))))) (suc (suc (suc (suc (zero)))))` |
| E5-5-5 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (zero)))))) (suc (suc (suc (suc (suc (zero))))))` |
| E5-5-6 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (zero)))))) (suc (suc (suc (suc (suc (suc (zero)))))))` |
| E5-6-0 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (suc (zero))))))) (zero)` |
| E5-6-1 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (suc (zero))))))) (suc (zero))` |
| E5-6-2 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (suc (zero))))))) (suc (suc (zero)))` |
| E5-6-3 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (suc (zero))))))) (suc (suc (suc (zero))))` |
| E5-6-4 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (suc (zero))))))) (suc (suc (suc (suc (zero)))))` |
| E5-6-5 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (suc (zero))))))) (suc (suc (suc (suc (suc (zero))))))` |
| E5-6-6 | E-law-plus | nf | 0 | PASS | `plus (suc (suc (suc (suc (suc (suc (zero))))))) (suc (suc (suc (suc (suc (suc (zero)))))))` |
| E06L | E-law-inline | nf | 0 | PASS | `let x : Nat = plus zero zero; suc x` |
| E07L | E-law-inline | nf | 0 | PASS | `let x : Nat = zero; plus x (suc zero)` |
| E08L | E-law-inline | nf | 0 | PASS | `let x : Nat = suc zero; let y : Nat = plus x x; y` |
| E09L | E-law-inline | nf | 0 | PASS | `let A : U = Nat; A` |
| E10L | E-law-inline | nf | 0 | PASS | `let x : Id Nat zero zero = refl; x` |
| E11L | E-law-inline | nf | 0 | PASS | `let f : Nat -> Nat = λ x . suc x; f (suc zero)` |
| E12 | E-law-det | nf | 0 | PASS | `Id Nat zero zero -> Nat` |
| E12 | E-law-det | nf | 0 | PASS | `Id Nat zero zero -> Nat` |
| E12 | E-law-det | nf | 0 | PASS | `Id Nat zero zero -> Nat` |
| E13 | E-law-type-rt | nf | 0 | PASS | `U` |
| E13 | E-law-type-rt | nf | 0 | PASS | `U` |
| E13 | E-law-type-rt | nf | 0 | PASS | `U` |

## Failure detail

### B13 (B-readme, mode=nf, exit=101)

```
input : let C : (A : U) -> Id Nat (suc zero) zero -> A = λ A (!); C Nat
expected exit 0 (no crash), got exit=101 stderr=
thread 'dpm-nbe' panicked at src\eval.rs:282:20:
eval: impossible (Bot)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: internal panic while elaborating the program

```

### B29 (B-gen, mode=nf, exit=0)

```
input : let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n refl refl . refl; H zero zero refl
exit=0 stdout=λ m n refl refl . refl
  :
Id Nat zero zero → Id Nat zero zero

```

### B30 (B-gen, mode=nf, exit=0)

```
input : let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = λ m n refl refl . refl; H zero
exit=0 stdout=λ m n refl refl . refl
  :
(n : Nat) → Id Nat n zero → Id Nat zero zero → Id Nat n zero

```

### D30 (D-err, mode=nf, exit=101)

```
input : let f : (n : Nat) -> Id Nat (suc n) zero -> Nat = λ n (!); f
expected exit 0 (no crash), got exit=101 stderr=
thread 'dpm-nbe' panicked at src\eval.rs:282:20:
eval: impossible (Bot)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
error: internal panic while elaborating the program

```

### E01a (E-law-roundtrip, mode=nf, exit=1)

```
input : let z : Nat = succ (succ zero); z
re-checking the printed normal form at its printed type failed or changed it: exit=1 stderr=<stdin>:1:15: |   | | 1 | let z : Nat = succ (succ zero); z |   |               ^ | unbound variable: succ |  out=
```

### E04a (E-law-roundtrip, mode=nf, exit=1)

```
input : let z : Nat → Nat = λ x . succ x; z
re-checking the printed normal form at its printed type failed or changed it: exit=1 stderr=<stdin>:1:27: |   | | 1 | let z : Nat → Nat = λ x . succ x; z |   |                           ^ | unbound variable: succ |  out=
```

### E06a (E-law-roundtrip, mode=nf, exit=1)

```
input : let z : (n : Nat) → Id Nat n zero → Id Nat zero zero → Id Nat n zero = λ m n refl refl . refl; z
re-checking the printed normal form at its printed type failed or changed it: exit=1 stderr=<stdin>:1:72: |   | | 1 | let z : (n : Nat) → Id Nat n zero → Id Nat zero zero → Id Nat n zero = λ m n refl refl . refl; z |   |                                                                        ^ | Too many patterns. |  out=
```

### E06b (E-law-roundtrip, mode=nf, exit=1)

```
input : let z : (n : Nat) → Id Nat n zero → Id Nat zero zero → Id Nat n zero = λ m n refl refl . refl; z
still fails after succ->suc mapping: exit=1 stderr=<stdin>:1:72: |   | | 1 | let z : (n : Nat) → Id Nat n zero → Id Nat zero zero → Id Nat n zero = λ m n refl refl . refl; z |   |                                                                        ^ | Too many patterns. |  out=
```

### E07a (E-law-roundtrip, mode=nf, exit=1)

```
input : let z : U = (A : U)(A' : U) → A'; z
re-checking the printed normal form at its printed type failed or changed it: exit=1 stderr=<stdin>:1:22: |   | | 1 | let z : U = (A : U)(A' : U) → A'; z |   |                      ^ | expected ':' |  out=
```

### E07b (E-law-roundtrip, mode=nf, exit=1)

```
input : let z : U = (A : U)(A' : U) → A'; z
still fails after succ->suc mapping: exit=1 stderr=<stdin>:1:22: |   | | 1 | let z : U = (A : U)(A' : U) → A'; z |   |                      ^ | expected ':' |  out=
```

