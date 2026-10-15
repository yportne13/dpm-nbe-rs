#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 5: pattern limits, grouped binders, odd error locations."""
import os
import subprocess
import sys

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


def line(label, text, mode="nf"):
    rc, out, err = run(text, mode)
    last = err.strip().split("\n")[-1] if err.strip() else ""
    col = err.split(":")[1] if err.startswith("<stdin>:") else ""
    print("%-40s rc=%-4s col=%-6s %-45s out=%r" % (
        label, rc, col, last[:45], out.strip()[:60]))
    return rc, out, err


print("### A. many patterns, flat arrow type: let f : (Nat ->)^n Nat = \\p0..pn . p0; f")
for n in (1, 2, 4, 8, 16, 32, 33, 64, 65, 100, 128, 200):
    ty = " -> ".join(["Nat"] * n)
    pats = " ".join("p%d" % i for i in range(n))
    line("n=%d patterns" % n, "let f : %s = \\%s . p0; f" % (ty, pats))

print()
print("### B. same but grouped binder in the type (one binder group)")
for n in (4, 32, 65, 100, 128, 200, 500, 1000):
    names = " ".join("p%d" % i for i in range(n))
    line("n=%d grouped" % n,
         "let f : (%s : Nat) -> Nat = \\%s . p0; f" % (names, names))

print()
print("### C. lambda patterns with a SINGLE arrow type (ill-typed, parse depth)")
for n in (1, 2, 4, 8, 16, 32, 64, 100):
    pats = " ".join("p%d" % i for i in range(n))
    line("n=%d pats / 1 arrow" % n, "let f : Nat -> Nat = \\%s . p0; f" % pats)

print()
print("### D. does the arrow-type itself cap out without a lambda?")
for n in (100, 200, 400, 500, 512):
    ty = " -> ".join(["Nat"] * n)
    line("arrow type n=%d, body zero" % n, "let f : %s = zero; f" % ty)

print()
print("### E. pattern-matching refinement: error location/message family")
cases = [
    ("refl-pattern, body var A",
     "let f : (A : U) -> Id A zero zero -> A = \\A refl . A; f"),
    ("refl-pattern, body var A (goal U)",
     "let f : (A : U) -> Id A zero zero -> U = \\A refl . A; f"),
    ("refl-pattern, body zero",
     "let f : (A : U) -> Id A zero zero -> A = \\A refl . zero; f"),
    ("refl-pattern, body refl",
     "let f : (A : U) -> Id A zero zero -> A = \\A refl . refl; f"),
    ("var-pattern, body var A",
     "let f : (A : U) -> Id A zero zero -> A = \\A r . A; f"),
    ("var-pattern, body zero",
     "let f : (A : U) -> Id A zero zero -> A = \\A r . zero; f"),
    ("refl-pattern on Id A a a",
     "let f : (A : U) (a : A) -> Id A a a -> A = \\A a refl . A; f"),
    ("refl-pattern, goal Nat",
     "let f : (A : U) -> Id A zero zero -> Nat = \\A refl . A; f"),
    ("refl-pattern, two refls",
     "let f : (A : U) -> Id A zero zero -> Id A zero zero -> A = \\A refl refl . A; f"),
    ("refl then absurd",
     "let f : (A : U) -> Id A zero zero -> Id Nat (suc zero) zero -> A = \\A refl (!); U"),
    ("absurd then refl",
     "let f : (A : U) -> Id Nat (suc zero) zero -> Id A zero zero -> A = \\A (!); U"),
]
for label, src in cases:
    rc, out, err = line(label, src)
    if err:
        print("      ERR: %s" % err.strip().replace("\n", " | ")[:300])

print()
print("### F. same-shape terms where the index is a variable")
cases2 = [
    ("Id A x y -> A, refl pattern",
     "let f : (A : U) (x y : A) -> Id A x y -> A = \\A x y refl . A; f"),
    ("Id A x y -> U, refl pattern",
     "let f : (A : U) (x y : A) -> Id A x y -> U = \\A x y refl . A; f"),
    ("Id Nat x y -> Nat, refl pattern",
     "let f : (x y : Nat) -> Id Nat x y -> Nat = \\x y refl . x; f"),
    ("Id Nat (plus x zero) x, refl",
     "let f : (x : Nat) -> Id Nat (plus x zero) x -> Nat = \\x refl . x; f"),
    ("Id Nat (plus x zero) x -> Nat, (!)",
     "let f : (x : Nat) -> Id Nat (plus x zero) (suc x) -> Nat = \\x (!); f"),
]
for label, src in cases2:
    rc, out, err = line(label, src)
    if err:
        print("      ERR: %s" % err.strip().replace("\n", " | ")[:300])
