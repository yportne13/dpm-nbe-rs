#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 10: the newest live build - new depth limit, the new Bot glyph, echo sizes."""
import os
import subprocess
import sys
import time

LIVE = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"
BIN = os.environ.get("DPM_BIN") or LIVE


def run(text, mode="nf", timeout=60.0):
    data = text.encode("utf-8") if isinstance(text, str) else text
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", e.stdout or b"", e.stderr or b""


def show(label, text, mode="nf", raw=False):
    rc, out, err = run(text, mode)
    o = out if raw else out.decode("utf-8", "replace").strip().replace("\n", " ")
    e = err.decode("utf-8", "replace").strip().split("\n")[-1] if err else ""
    print("%-44s rc=%-4s out=%-8d %s" % (label, rc, len(out), (o or e)[:80]))
    return rc, out, err


print("### A. the new Bot glyph in printed output")
C = "let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C"
rc, out, err = run(C)
print("   raw stdout:", repr(out))
print("   raw stderr:", repr(err))
bot = out.decode("utf-8", "replace").split("\n  :\n")[0]
print("   printed nf:", repr(bot))
show("feed the printed nf back", bot)
show("the glyph alone", "\u22a5")
show("let t : Nat = \u22a5; t", "let t : Nat = \u22a5; t")
show("bottom as a binder name", "let \u22a5 : Nat = zero; \u22a5")
show("type of the absurd lambda (type mode)",
     "let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C", "type")

print()
print("### B. new depth limit bisection (live build)")
for n in (1000, 1023, 1024, 1025, 1500, 2000, 2048, 3000, 4000):
    show("parens n=%d" % n, "(" * n + "zero" + ")" * n)
for n in (1024, 1025, 2000, 2048, 3000):
    show("arrow chain n=%d" % n, " -> ".join(["Nat"] * n))

print()
print("### C. error-echo sizes on the live build")
for label, src in [("100k ident", "a" * 100000), ("1M ident", "a" * 1000000),
                   ("1M junk after zero", "zero" + "$" * 1000000),
                   ("10MB spaces", b" " * 10000000)]:
    rc, out, err = run(src)
    print("%-22s rc=%-4s stderr=%-10d (input %d)" % (
        label, rc, len(err), len(src.encode("utf-8") if isinstance(src, str) else src)))

print()
print("### D. remaining headline defects on the live build")
show("D2 conversion self-equality",
     "let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f")
rc, out, err = run("let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f")
print("   full stderr:", repr(err[:400]))
show("D4 stack overflow k=17",
     "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 17 + "suc zero" + ")" * 17, timeout=120)
