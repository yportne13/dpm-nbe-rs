data Nat : U where zero : Nat; suc : (n : Nat) -> Nat;

let plus : Nat -> Nat -> Nat =
  λ m n . elim Nat (λ _ . Nat) n (λ _ r . suc r) m;

let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =
  λ A a b c refl refl . refl;

let cong : (A B : U) (f : A -> B) (a b : A) -> Id A a b -> Id B (f a) (f b) =
  λ A B f a b refl . refl;

let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =
  λ m n refl refl . refl;

let C : (A : U) -> Id Nat (suc zero) zero -> A =
  λ A (!);

U
