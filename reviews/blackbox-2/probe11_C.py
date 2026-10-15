#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 11: newest build (C) - where is the depth limit now, and is the
boundary message still wrong?  Plus the Bot glyph and deep round trip."""
import os
import subprocess

BIN = os.environ.get("DPM_BIN") or r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"


def run(text, mode="nf", timeout=90.0):
    data = text.encode("utf-8") if isinstance(text, str) else text
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", e.stdout or b"", e.stderr or b""


def last(err):
    return err.decode("utf-8", "replace").strip().split("\n")[-1] if err else ""


print("### depth limit on build C")
for n in (100, 200, 255, 256, 257, 300, 512, 513, 1024, 1025, 2000):
    rc, out, err = run("(" * n + "zero" + ")" * n)
    print("  parens n=%-5d rc=%-4s col=%-6s %s" % (
        n, rc, (err.split(b":")[1].decode() if err.startswith(b"<stdin>:") else ""), last(err)[:60]))
for n in (255, 256, 257, 512, 1024, 2048):
    rc, out, err = run(" -> ".join(["Nat"] * n))
    print("  arrows n=%-5d rc=%-4s %s" % (n, rc, last(err)[:60]))

print()
print("### Bot glyph round trip on build C")
C = "let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C"
rc, out, err = run(C)
print("  nf rc=%s stdout=%r" % (rc, out[:90]))
if rc == 0:
    nf = out.decode("utf-8", "replace").split("\n  :\n")[0]
    rc2, o2, e2 = run(nf)
    print("  feed back %r -> rc=%s %s" % (nf, rc2, last(e2)[:70]))

print()
print("### deep printed nf round trip on build C (k=10 doubling -> 1024 successors)")
src = "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 10 + "suc zero" + ")" * 10
rc, out, err = run(src)
print("  step1 rc=%s len=%d" % (rc, len(out)))
if rc == 0:
    nf = out.decode("utf-8", "replace").split("\n  :\n")[0]
    rc2, o2, e2 = run(nf)
    print("  step2 rc=%s identical=%s %s" % (rc2, o2 == out, last(e2)[:70]))

print()
print("### stack overflow still there on build C?")
rc, out, err = run("let d : Nat -> Nat = \\n . plus n n; " + "d (" * 17 + "suc zero" + ")" * 17, timeout=120)
print("  k=17 rc=%s (0x%08X) stderr=%r" % (rc, rc & 0xFFFFFFFF if isinstance(rc, int) else 0, err[:60]))
