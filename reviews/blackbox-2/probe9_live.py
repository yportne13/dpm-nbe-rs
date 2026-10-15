#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 9: delta against the *current* live build (whatever its hash is).

Checks which findings still reproduce and whether the fixes introduced new
problems (e.g. removal of the parser depth guard -> stack overflow).
"""
import os
import subprocess
import sys
import time

LIVE = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"
HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("DPM_BIN") or LIVE


def run(text, mode="nf", timeout=30.0):
    data = text.encode("utf-8") if isinstance(text, str) else text
    t0 = time.perf_counter()
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        rc, out, err = p.returncode, p.stdout, p.stderr
        to = False
    except subprocess.TimeoutExpired as e:
        rc, out, err, to = None, e.stdout or b"", e.stderr or b"", True
    return rc, out, err, to, (time.perf_counter() - t0) * 1000


def show(label, text, mode="nf", timeout=30.0, w=70):
    rc, out, err, to, ms = run(text, mode, timeout)
    last = err.decode("utf-8", "replace").strip().split("\n")[-1] if err else ""
    o = out.decode("utf-8", "replace").strip().replace("\n", " ")
    tag = "TIMEOUT" if to else ("rc=%s" % rc)
    print("%-42s %-10s %8.0fms out=%-6d %s" % (label, tag, ms, len(out), (o or last)[:w]))
    return rc, out, err, to


print("### A. previously-fixed items, on the live build")
show("D1 absurd lambda in nf", "let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C")
show("D3 what does nf print now?", "suc zero")
show("D3b feed printed form back", "succ zero")
show("D3c is 'succ' a keyword now?", "let succ : Nat = suc zero; succ")
show("D5 512 parens", "(" * 512 + "zero" + ")" * 512)
show("D5b 513 parens", "(" * 513 + "zero" + ")" * 513)
show("D7 unterminated comment", "zero {- oops")
show("D9 doubling k=10 (1024 sucs) -> printed nf", "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 10 + "suc zero" + ")" * 10)

print()
print("### B. REGRESSION HUNT: is the parser depth guard still there?")
for n in (512, 513, 1000, 5000, 10000, 50000, 100000, 300000):
    show("parens n=%d" % n, "(" * n + "zero" + ")" * n, timeout=60)
for n in (513, 1000, 5000, 50000):
    show("arrow chain n=%d" % n, " -> ".join(["Nat"] * n), timeout=60)
for n in (1000, 10000, 100000):
    body = "zero"
    for _ in range(n):
        body = "suc (" + body + ")"
    show("suc tower n=%d" % n, body, timeout=60)
for n in (10000,):
    show("flat app chain n=%d" % n, "let f : Nat -> Nat = \\n . n; " + "f " * n + "zero", timeout=60)
    show("paren app chain n=%d" % n, "let f : Nat -> Nat = \\n . n; " + "f (" * n + "zero" + ")" * n, timeout=60)

print()
print("### C. remaining findings on the live build")
show("D2 conversion self-equality", "let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f")
show("D2b var vs own type", "let k : (n m : Nat) (p : Id Nat (plus n m) (plus n m)) -> Id Nat (plus n m) (plus n m) = \\n m p . p; k")
show("D6 invalid utf8", b"zero\xff")
show("D8 1MB identifier echo", "a" * 1000000)
show("D10 BOM", "\ufeffzero")
show("D11 underscore term", "let x : Nat = zero; _")
show("D11b underscore binder", "let f : (A : U) -> A -> A = \\A _ . A; f")
show("D4 stack overflow k=17", "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 17 + "suc zero" + ")" * 17, timeout=90)

print()
print("### D. round trip on the live build (feed printed nf back)")
for label, src in [("plus", "plus (suc zero) (suc zero)"),
                   ("big", "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 10 + "suc zero" + ")" * 10),
                   ("lambda", "let f : (A : U) -> A -> A = \\A x . x; f")]:
    rc, out, err, to, ms = run(src, "nf", 60)
    if rc != 0:
        print("%-10s step1 rc=%s" % (label, rc)); continue
    nf = out.decode("utf-8", "replace").split("\n  :\n")[0]
    rc2, out2, err2, to2, ms2 = run(nf, "nf", 60)
    same = out2 == out
    print("%-10s step2 rc=%-4s identical=%-5s len=%-7d %s" % (
        label, rc2, same, len(nf), "" if same else err2.decode("utf-8", "replace").strip().split("\n")[-1][:70]))
