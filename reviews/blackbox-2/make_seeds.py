#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Write the anomalous-input seed corpus used by fuzz round 2."""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
D = os.path.join(HERE, "seeds")
os.makedirs(D, exist_ok=True)

SEEDS = {
    "panic_absurd_min": "let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C\n",
    "panic_absurd_readme": ("let C : (A : U) -> Id Nat (suc zero) zero -> A =\n"
                            "  \u03bb A (!);\nC\n"),
    "panic_absurd_in_type": ("let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!);\n"
                             "let x : Id ((A : U) -> Id Nat (suc zero) zero -> A) C C = refl; x\n"),
    "conv_self_equal": "let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f\n",
    "conv_step": "let f : (n m : Nat) -> Id Nat (plus (suc n) m) (suc (plus n m)) = \\n m . refl; f\n",
    "conv_neutral_arg": "let f : (g : Nat -> Nat) (n m : Nat) -> Id Nat (g (plus n m)) (g (plus n m)) = \\g n m . refl; f\n",
    "conv_var_own_type": ("let k : (n m : Nat) (p : Id Nat (plus n m) (plus n m)) -> "
                          "Id Nat (plus n m) (plus n m) = \\n m p . p; k\n"),
    "depth_511": "(" * 511 + "zero" + ")" * 511 + "\n",
    "depth_512": "(" * 512 + "zero" + ")" * 512 + "\n",
    "depth_513": "(" * 513 + "zero" + ")" * 513 + "\n",
    "depth_arrows_512": " -> ".join(["Nat"] * 512) + "\n",
    "depth_arrows_513": " -> ".join(["Nat"] * 513) + "\n",
    "double_k8": "let d : Nat -> Nat = \\n . plus n n; d (d (d (d (d (d (d (d (suc zero))))))))\n",
    "double_k12": "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 12 + "suc zero" + ")" * 12 + "\n",
    "readme_full": ("let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c =\n"
                    "  \u03bb A a b c refl refl . refl;\n\n"
                    "let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero =\n"
                    "  \u03bb m n refl refl . refl;\n\n"
                    "let C : (A : U) -> Id Nat (suc zero) zero -> A =\n"
                    "  \u03bb A (!);\n\nU\n"),
    "grouped_binder_100": "let f : (%s : Nat) -> Nat = \\%s . p0; f\n" % (
        " ".join("p%d" % i for i in range(100)), " ".join("p%d" % i for i in range(100))),
    "unicode_line": "\u03b1" * 300 + " )\n",
    "unicode_id": "let \u03b1\u03b2 : Nat = zero; \u03b1\u03b2\n",
    "bom": "\ufeffzero\n",
    "unterminated_comment": "zero {- unterminated\n",
    "nested_comment": "{- {- -} -} zero\n",
    "zero_junk": "zero" + "$" * 2000 + "\n",
    "impossible_refl": "let f : (A : U) -> Id Nat (suc zero) zero -> A = \\A refl . A; f\n",
    "absurd_open": "let f : (x : Nat) -> Id Nat (plus x zero) (suc x) -> Nat = \\x (!); f\n",
    "many_patterns": "let f : (p0 p1 p2 : Nat) -> Nat = \\p0 p1 p2 . p0; f\n",
    "lets_200": "".join("let x : Nat = suc x; " for _ in range(200)) + "x\n",
    "shadow_kw": "let suc : Nat = zero; let zero : Nat = suc; zero\n",
    "succ_ident": "let succ : Nat = suc zero; succ\n",
    "arrow_lambda": "let f : " + " -> ".join(["Nat"] * 20) + " = \\" + " ".join(
        "p%d" % i for i in range(20)) + " . p0; f\n",
}

for name, text in SEEDS.items():
    with open(os.path.join(D, name + ".txt"), "w", encoding="utf-8", newline="") as fh:
        fh.write(text)

# binary seeds
with open(os.path.join(D, "utf8_bad.txt"), "wb") as fh:
    fh.write(b"zero\xff\n")
with open(os.path.join(D, "nul.txt"), "wb") as fh:
    fh.write(b"zero\x00\n")

print("wrote", len(SEEDS) + 2, "seeds to", D)
