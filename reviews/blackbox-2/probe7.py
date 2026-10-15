#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 7: definitional equality of stuck `plus` forms (suspected conversion bug)."""
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


def show(label, text, mode="nf", width=90):
    rc, out, err = run(text, mode)
    last = err.strip().split("\n")[-1] if err.strip() else ""
    print("%-56s rc=%-4s %s" % (label, rc, (out.strip() or last).replace("\n", " ")[:width]))
    return rc, out, err


print("### A. what do stuck plus forms print as (nf of a lambda body)?")
show("plus (suc n) m", "let f : (n m : Nat) -> Nat = \\n m . plus (suc n) m; f")
show("plus n m", "let f : (n m : Nat) -> Nat = \\n m . plus n m; f")
show("plus n zero", "let f : (n : Nat) -> Nat = \\n . plus n zero; f")
show("plus zero n", "let f : (n : Nat) -> Nat = \\n . plus zero n; f")
show("plus (suc n) zero", "let f : (n : Nat) -> Nat = \\n . plus (suc n) zero; f")
show("plus n (suc m)", "let f : (n m : Nat) -> Nat = \\n m . plus n (suc m); f")
show("plus (suc (suc n)) m", "let f : (n m : Nat) -> Nat = \\n m . plus (suc (suc n)) m; f")

print()
print("### B. refl against definitionally-equal stuck forms (expect: OK)")
cases = [
    ("identical: plus n m = plus n m",
     "let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f"),
    ("identical: plus (suc n) m = itself",
     "let f : (n m : Nat) -> Id Nat (plus (suc n) m) (plus (suc n) m) = \\n m . refl; f"),
    ("step:  plus (suc n) m = suc (plus n m)",
     "let f : (n m : Nat) -> Id Nat (plus (suc n) m) (suc (plus n m)) = \\n m . refl; f"),
    ("step swapped: suc (plus n m) = plus (suc n) m",
     "let f : (n m : Nat) -> Id Nat (suc (plus n m)) (plus (suc n) m) = \\n m . refl; f"),
    ("two steps: plus (suc (suc n)) m = suc (suc (plus n m))",
     "let f : (n m : Nat) -> Id Nat (plus (suc (suc n)) m) (suc (suc (plus n m))) = \\n m . refl; f"),
    ("step with zero: plus (suc n) zero = suc (plus n zero)",
     "let f : (n : Nat) -> Id Nat (plus (suc n) zero) (suc (plus n zero)) = \\n . refl; f"),
    ("zero rule: plus zero n = n",
     "let f : (n : Nat) -> Id Nat (plus zero n) n = \\n . refl; f"),
    ("zero rule under suc: plus zero (suc n) = suc n",
     "let f : (n : Nat) -> Id Nat (plus zero (suc n)) (suc n) = \\n . refl; f"),
    ("nested: plus zero (plus (suc n) m) = suc (plus n m)",
     "let f : (n m : Nat) -> Id Nat (plus zero (plus (suc n) m)) (suc (plus n m)) = \\n m . refl; f"),
    ("suc congruence: suc (plus n m) = suc (plus n m)",
     "let f : (n m : Nat) -> Id Nat (suc (plus n m)) (suc (plus n m)) = \\n m . refl; f"),
    ("plus on both: plus (plus zero n) m = plus n m",
     "let f : (n m : Nat) -> Id Nat (plus (plus zero n) m) (plus n m) = \\n m . refl; f"),
    ("plus m zero = m (README H shape)",
     "let f : (m : Nat) -> Id Nat (plus m zero) m = \\m . refl; f"),
    ("README H",
     "let H : (m n : Nat) -> Id Nat n (plus m zero) -> Id Nat m zero -> Id Nat n zero = "
     "\\m n refl refl . refl; H"),
]
for label, src in cases:
    rc, out, err = show(label, src)
    if err.strip():
        print("      ERR: %s" % err.strip().replace("\n", " | ")[:260])

print()
print("### C. conversion under two binders (does the *number* of neutrals matter?)")
cases2 = [
    ("one binder: plus (suc n) zero vs suc (plus n zero)",
     "let f : (n : Nat) -> Id Nat (plus (suc n) zero) (suc (plus n zero)) = \\n . refl; f"),
    ("three binders: plus (suc n) m with extra var",
     "let f : (n m k : Nat) -> Id Nat (plus (suc n) m) (suc (plus n m)) = \\n m k . refl; f"),
    ("plus with 2 vars on the right: plus (suc n) (plus m k)",
     "let f : (n m k : Nat) -> Id Nat (plus (suc n) (plus m k)) (suc (plus n (plus m k))) = \\n m k . refl; f"),
    ("convertibility discovered by let: X := plus (suc n) m",
     "let f : (n m : Nat) -> Nat = \\n m . plus (suc n) m; "
     "let g : (n m : Nat) -> Id Nat (plus (suc n) m) (suc (plus n m)) = \\n m . refl; g"),
]
for label, src in cases2:
    rc, out, err = show(label, src)
    if err.strip():
        print("      ERR: %s" % err.strip().replace("\n", " | ")[:260])

print()
print("### D. is it plus-specific? Id/other primitives")
show("Id index conversion: Id (Id Nat zero zero) refl refl",
     "let x : Id (Id Nat zero zero) refl refl = refl; x")
show("transport-free: plus (suc zero) m vs suc m (concrete first arg)",
     "let f : (m : Nat) -> Id Nat (plus (suc zero) m) (suc m) = \\m . refl; f")
show("plus (suc zero) (suc m) vs suc (suc m)",
     "let f : (m : Nat) -> Id Nat (plus (suc zero) (suc m)) (suc (suc m)) = \\m . refl; f")
show("plus (suc n) (suc m) vs suc (plus n (suc m))",
     "let f : (n m : Nat) -> Id Nat (plus (suc n) (suc m)) (suc (plus n (suc m))) = \\n m . refl; f")
