#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 8: scope of the 'neutral not convertible to itself' bug."""
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("DPM_BIN") or os.path.join(HERE, "bin", "pinned-33052DDC.exe")


def run(text, mode="nf", timeout=20.0):
    data = text.encode("utf-8") if isinstance(text, str) else text
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return "TIMEOUT", "", ""


def show(label, text, mode="nf", width=76):
    rc, out, err = run(text, mode)
    last = err.strip().split("\n")[-1] if err.strip() else ""
    print("%-58s rc=%-4s %s" % (label, rc, (out.strip() or last).replace("\n", " ")[:width]))
    return rc, out, err


CASES = [
    # --- identity types whose sides are neutral applications
    ("plus n m = plus n m (both neutral primitives)",
     "let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f"),
    ("plus n n = plus n n",
     "let f : (n : Nat) -> Id Nat (plus n n) (plus n n) = \\n . refl; f"),
    ("user fn neutral: g n = g n",
     "let f : (g : Nat -> Nat) (n : Nat) -> Id Nat (g n) (g n) = \\g n . refl; f"),
    ("user fn neutral, 2 args: g n m = g n m",
     "let f : (g : Nat -> Nat -> Nat) (n m : Nat) -> Id Nat (g n m) (g n m) = \\g n m . refl; f"),
    ("g (plus n m) = g (plus n m)",
     "let f : (g : Nat -> Nat) (n m : Nat) -> Id Nat (g (plus n m)) (g (plus n m)) = \\g n m . refl; f"),
    ("bare var: n = n", "let f : (n : Nat) -> Id Nat n n = \\n . refl; f"),
    ("suc n = suc n", "let f : (n : Nat) -> Id Nat (suc n) (suc n) = \\n . refl; f"),
    ("plus zero n = plus zero n",
     "let f : (n : Nat) -> Id Nat (plus zero n) (plus zero n) = \\n . refl; f"),
    ("plus n zero = plus n zero",
     "let f : (n : Nat) -> Id Nat (plus n zero) (plus n zero) = \\n . refl; f"),
    # --- type-level (U) neutral equality
    ("A -> B = A -> B", "let f : (A B : U) -> Id U (A -> B) (A -> B) = \\A B . refl; f"),
    ("A = A", "let f : (A : U) -> Id U A A = \\A . refl; f"),
    ("(x : A) -> B x = itself",
     "let f : (A : U) (B : A -> U) -> Id U ((x : A) -> B x) ((x : A) -> B x) = \\A B . refl; f"),
    ("Nat -> Nat = Nat -> Nat (closed)", "let x : Id U (Nat -> Nat) (Nat -> Nat) = refl; x"),
    # --- same, but with a variable of the identity type (no refl)
    ("variable of type Id Nat (plus n m) (plus n m)",
     "let k : (n m : Nat) (p : Id Nat (plus n m) (plus n m)) -> Id Nat (plus n m) (plus n m) "
     "= \\n m p . p; k"),
    ("function returning p, plus neutral",
     "let k : (n m : Nat) -> Id Nat (plus n m) (plus n m) -> Nat = \\n m p . zero; k"),
    ("apply a variable to a neutral plus",
     "let k : (g : Nat -> Nat) (n m : Nat) -> Nat = \\g n m . g (plus n m); k"),
    # --- conversion triggered by application rather than refl
    ("expected/actual conversion: check arg of a Pi",
     "let g : (n m : Nat) -> Nat = \\n m . zero; "
     "let h : (n m : Nat) -> Nat -> Nat = \\n m k . k; h"),
    ("conversion needed in a check: id (plus n m) where id expects Nat",
     "let id : Nat -> Nat = \\x . x; let f : (n m : Nat) -> Nat = \\n m . id (plus n m); f"),
    # --- README-ish regressions
    ("trans with neutral plus",
     "let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = "
     "\\A a b c refl refl . refl; trans"),
    ("plus n m symmetric-ish (should fail, legitimately)",
     "let f : (n m : Nat) -> Id Nat (plus n m) (plus m n) = \\n m . refl; f"),
]

for label, src in CASES:
    rc, out, err = show(label, src)
    if err.strip():
        e = err.strip().replace("\n", " | ")
        if "equals to" in e:
            print("      MSG: %s" % e[e.index("expecting"):][:200])
