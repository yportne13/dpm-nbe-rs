#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 3: bisect limits, hunt UTF-8 slicing panics, real exponential blowup."""
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
PINNED = os.path.join(HERE, "bin", "pinned-33052DDC.exe")
BIN = os.environ.get("DPM_BIN") or PINNED


def run(text, mode="nf", timeout=15.0):
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


def brief(b, n=160):
    s = b.decode("utf-8", "replace")
    return s if len(s) <= n else s[:n] + "...<%d more>" % (len(s) - n)


def show(label, text, mode="nf", timeout=15.0):
    rc, out, err, to, ms = run(text, mode, timeout)
    tag = "TIMEOUT" if to else ("rc=%s" % rc)
    tail = brief(err.decode("utf-8", "replace").strip().split("\n")[-1].encode("utf-8") if err else b"", 120)
    print("%-46s %-9s %7.1fms out=%-3d err=%-7d last=%s" % (
        label, tag, ms, len(out), len(err), tail))
    if b"panic" in err or (to is False and rc not in (0, 1, 2, 101)):
        print("    FULL ERR:", brief(err, 600))
        print("    FULL OUT:", brief(out, 300))
    return rc, out, err, to, ms


print("### A. multibyte error rendering (UTF-8 slicing panic hunt)")
show("300 greek + close paren", "\u03b1 " * 300 + ")")
show("600 greek atoms + paren", "\u03b1" * 600 + ")")
show("600 cjk + paren", "\u4e2d" * 600 + ")")
show("emoji x300 + paren", "\U0001f600" * 300 + ")")
show("greek ident then junk", "let \u03b1 : Nat = zero; \u03b1 zero zero")
show("nested greek lambda err", "let f : Nat -> Nat = \u03bb \u03b1 . \u03b1; f zero zero")
show("long greek line err", "\u03b1" * 2000 + " )")
show("combining chars + err", "e\u0301" * 400 + " )")
show("mixed multibyte parens", "(" * 300 + "\u03b1" * 300 + ")" * 300 + " )")
show("BOM then greek", "\ufeff\u03b1")
show("zwnj heavy", "\u200c" * 400 + " )")

print()
print("### B. depth limit bisection (nested parens)")
lo = None
for n in (498, 499, 500, 501, 502, 505, 510, 511, 512, 513, 514, 520, 600):
    rc, out, err, to, ms = show("parens n=%d" % n, "(" * n + "zero" + ")" * n)
    last = err.decode("utf-8", "replace").strip().split("\n")[-1]
    print("      -> msg: %s" % last)

print()
print("### C. pattern-count limit bisection (lambda)")
for n in (100, 400, 499, 500, 501, 512, 513, 600, 1000):
    ty = " -> ".join(["Nat"] * n)
    pats = " ".join("p%d" % i for i in range(n))
    rc, out, err, to, ms = show("arrows/patterns n=%d" % n,
                                "let f : %s = \\%s . p0; f" % (ty, pats))

print()
print("### D. arrow-chain limit at top level (no let)")
for n in (500, 501, 512, 513, 1000, 2000, 5000):
    show("top arrow chain n=%d" % n, " -> ".join(["Nat"] * n))

print()
print("### E. exponential blowup: d(d(...(suc zero)))  [2^k succs]")
for k in (8, 12, 16, 18, 20, 22, 24, 26):
    body = "suc zero"
    for _ in range(k):
        body = "d (" + body + ")"
    rc, out, err, to, ms = show("doubling k=%d (~2^%d)" % (k, k),
                                "let d : Nat -> Nat = \\n . plus n n; " + body,
                                timeout=20.0)
    print("      input=%d bytes out=%d bytes" % (len(body) + 40, len(out)))

print()
print("### F. huge echo: error at col 1 on one giant line")
for n in (100000, 1000000):
    rc, out, err, to, ms = show("junk col1 %d chars" % n, "zero" + " " * n, timeout=20.0)
    print("      stderr bytes=%d (input %d)" % (len(err), n + 4))

print()
print("### G. absurd-pattern panic variants (type mode vs nf mode)")
C = "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); %s"
for term in ("C", "C Nat", "C Nat refl", "(\\A (!))", "zero"):
    show("absurd final=%s [nf]" % term, C % term, "nf")
    show("absurd final=%s [type]" % term, C % term, "type")

print()
print("### H. absurd pattern on inhabited type")
show("absurd on A -> A (used)", "let f : (A : U) -> A -> A = \\A (!); f", "nf")
show("absurd on A -> A (unused)", "let f : (A : U) -> A -> A = \\A (!); U", "nf")
show("absurd in type position", "let f : (A : U) -> A -> A = \\A (!); Id Nat (plus zero zero) zero", "nf")
show("absurd nested in pair-like", "let f : (A : U) -> A -> A = \\A (!); let g : Nat = zero; f", "nf")
