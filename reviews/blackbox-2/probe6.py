#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 6: the Bot/absurd-lambda panic family + a few remaining edge cases."""
import os
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("DPM_BIN") or os.path.join(HERE, "bin", "pinned-33052DDC.exe")

T = "(A : U) -> Id Nat (suc zero) zero -> A"
DECL = "let C : %s = \\A (!); " % T


def run(text, mode="nf", timeout=20.0):
    data = text.encode("utf-8") if isinstance(text, str) else text
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return "TIMEOUT", "", ""


def show(label, text, mode="nf"):
    rc, out, err = run(text, mode)
    last = err.strip().split("\n")[-1] if err.strip() else ""
    print("%-52s [%-4s] rc=%-4s out=%-28r %s" % (label, mode, rc, out.strip()[:28], last[:70]))
    return rc, out, err


print("### A. Bot value appearing in the printed normal form / type")
show("C as final term", DECL + "C")
show("C as final term", DECL + "C", "type")
show("Id T C C (type containing C)", DECL + "Id (%s) C C" % T)
show("Id T C C", DECL + "Id (%s) C C" % T, "type")
show("let x : Id T C C = refl; x", DECL + "let x : Id (%s) C C = refl; x" % T)
show("let x : Id T C C = refl; x", DECL + "let x : Id (%s) C C = refl; x" % T, "type")
show("C applied to Nat, result neutral?", DECL + "let h : %s -> Nat = \\f . zero; h C" % T)
show("C applied inside a third-party type", DECL + "let h : %s -> Nat = \\f . zero; h" % T)
show("C shadowed by a Nat value", DECL + "let C : Nat = zero; C")
show("C under an unused lambda", DECL + "let g : Nat -> Nat = \\n . n; g zero")
show("C returned by a function", DECL + "let g : Nat -> %s = \\n . C; g Nat" % T)
show("C returned by a function (fn itself)", DECL + "let g : Nat -> %s = \\n . C; g" % T)
show("two absurd lambdas", DECL + "let D : %s = \\A (!); Id (%s) C D" % (T, T))
show("absurd lambda with value pattern too",
     "let C : (A : U) -> Nat -> Id Nat (suc zero) zero -> A = \\A n (!); C")
show("absurd lambda, Nat -> Bot direct",
     "let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C")

print()
print("### B. panic reproducibility / exit code stability")
for i in range(3):
    rc, out, err = run(DECL + "C")
    print("  run %d: rc=%s stderr_len=%d stdout_empty=%s" % (i, rc, len(err), out == ""))
print("  full stderr:", repr(run(DECL + "C")[2]))

print()
print("### C. deeply nested let / shadowing exactness")
show("same name 300x", "".join("let x : Nat = suc zero; " for _ in range(300)) + "x")
show("chain of 300 dependent lets",
     "let v0 : Nat = zero; " + "".join("let v%d : Nat = suc v%d; " % (i, i - 1)
                                       for i in range(1, 300)) + "v299")
show("300 lets all in scope",
     "".join("let v%d : Nat = suc zero; " % i for i in range(300)) + "v0")
show("redefinition of keyword as let name",
     "let suc : Nat = zero; let zero : Nat = suc; zero")

print()
print("### D. conversions and definitional equality corners")
show("plus n zero = n (refl)", "let f : (n : Nat) -> Id Nat (plus n zero) n = \\n . refl; f")
show("plus zero n = n (refl)", "let f : (n : Nat) -> Id Nat (plus zero n) n = \\n . refl; f")
show("plus n zero vs n in TYPE", "Id Nat (plus (suc zero) zero) (suc zero)")
show("plus (suc n) m = suc (plus n m) via refl",
     "let f : (n m : Nat) -> Id Nat (plus (suc n) m) (suc (plus n m)) = \\n m . refl; f")
show("Id U Nat Nat refl", "let x : Id U Nat Nat = refl; x")
show("refl at U level", "let x : Id U U U = refl; x")
show("U : U ?", "let x : U = U; x")
show("Id U U U type", "Id U U U")
show("Id U (Id U U U) (Id U U U)",
     "let x : Id U (Id U U U) (Id U U U) = refl; x")
