#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Reproduces every confirmed defect / hardening item from scratch and writes
FINDINGS.txt.  Each entry: id, severity, command, exit code, stdout, stderr.

Run:  python repro.py
"""
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
LIVE = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"
PINNED = os.path.join(HERE, "bin", "pinned-33052DDC.exe")
BIN = os.environ.get("DPM_BIN") or (PINNED if os.path.exists(PINNED) else LIVE)


def run(data, mode, timeout=60.0):
    if isinstance(data, str):
        data = data.encode("utf-8")
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, p.stdout, p.stderr, None
    except subprocess.TimeoutExpired:
        return None, b"", b"", "timeout"


def cap(b, n=600):
    s = b.decode("utf-8", "replace")
    return s if len(s) <= n else s[:n] + "...<%d bytes total>" % len(b)


FINDINGS = [
    # (id, severity, title, mode, program bytes/str, note)
    ("D1", "HIGH",
     "nf panics (Rust panic, exit 101) whenever the printed normal form contains a "
     "lambda with an absurd pattern",
     "nf", "let C : Nat -> Id Nat (suc zero) zero -> Nat = \\n (!); C",
     "README's own C definition, with C instead of U as the final term"),
    ("D1b", "HIGH", "same panic via a type that contains the absurd lambda", "nf",
     "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); "
     "Id ((A : U) -> Id Nat (suc zero) zero -> A) C C", None),
    ("D1c", "HIGH", "same panic in `type` mode when the *type* mentions the absurd lambda", "type",
     "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); "
     "let x : Id ((A : U) -> Id Nat (suc zero) zero -> A) C C = refl; x", None),
    ("D1d", "HIGH", "type mode alone is fine for the plain case (panic is in quoting, not checking)",
     "type", "let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); C", None),

    ("D2", "HIGH",
     "conversion checker is not reflexive for stuck primitive (`plus`) neutrals: "
     "two textually identical types are judged unequal, and the diagnostic prints "
     "the same type twice",
     "nf", "let f : (n m : Nat) -> Id Nat (plus n m) (plus n m) = \\n m . refl; f",
     "the two indices are byte-identical in the message"),
    ("D2b", "HIGH", "even a variable checked against its own type is rejected", "nf",
     "let k : (n m : Nat) (p : Id Nat (plus n m) (plus n m)) -> "
     "Id Nat (plus n m) (plus n m) = \\n m p . p; k", None),
    ("D2c", "HIGH", "any neutral spine containing a stuck plus is affected", "nf",
     "let f : (g : Nat -> Nat) (n m : Nat) -> Id Nat (g (plus n m)) (g (plus n m)) = "
     "\\g n m . refl; f", None),
    ("D2d", "HIGH", "plain variable neutrals are fine (bounds the bug to primitive neutrals)", "nf",
     "let f : (g : Nat -> Nat) (n : Nat) -> Id Nat (g n) (g n) = \\g n . refl; f", None),
    ("D2e", "HIGH", "a plus-free Pi type compares fine (bug is primitive-specific)", "nf",
     "let f : (A B : U) -> Id U (A -> B) (A -> B) = \\A B . refl; f", None),

    ("D3", "HIGH",
     "printer emits `succ`, parser only accepts `suc`: no nf output that contains a "
     "successor can be fed back in",
     "nf", "suc (suc zero)", "prints `succ (succ zero)`; feeding that back is an error"),
    ("D3b", "HIGH", "feeding the printed normal form back", "nf", "succ (succ zero)", None),
    ("D3c", "HIGH", "`succ` is a legal user identifier, so the printed name is shadowable", "nf",
     "let succ : Nat = suc zero; succ", None),
    ("D3d", "HIGH", "the README documents the unparseable spelling `succ`", "nf",
     "plus (suc zero) (suc zero)", None),
    ("D9", "MEDIUM",
     "printed normal forms deeper than the parser's 512-frame limit cannot be re-imported",
     "nf", "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 10 + "suc zero" + ")" * 10,
     "prints a 1024-deep tower; that text re-parsed gives 'expression nesting is too deep'"),

    ("D4", "HIGH",
     "112-byte program -> STATUS_STACK_OVERFLOW (0xC00000FD); the process spends ~25 s of "
     "wall clock (0.2 s CPU) dying instead of reporting a clean error",
     "nf", "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 17 + "suc zero" + ")" * 17,
     "k=16 completes in 3 s; k=17 aborts"),
    ("D12", "LOW",
     "normalisation+printing is markedly superlinear (2^14: 80 ms, 2^16: 3.0 s, 50 MB)",
     "nf", "let d : Nat -> Nat = \\n . plus n n; " + "d (" * 16 + "suc zero" + ")" * 16, None),

    ("D5", "LOW", "depth guard fires with a wrong message exactly at the boundary (512 parens)",
     "nf", "(" * 512 + "zero" + ")" * 512,
     "511 parens parse fine; 512 gives \"'zero' is a keyword\"; 513 gives the real message"),
    ("D5b", "LOW", "the real depth message", "nf", "(" * 513 + "zero" + ")" * 513, None),

    ("D6", "LOW", "stdin decoding failures exit 2, while all other errors exit 1", "nf",
     b"zero\xff", "contract in README: errors exit 1"),

    ("D7", "LOW", "unterminated block comment is accepted silently", "nf", "zero {- oops", None),
    ("D7b", "LOW", "same for a comment that never terminates at all", "nf", "{- oops", None),

    ("D8", "LOW",
     "the error renderer echoes the whole remaining source line, so stderr is ~2x the "
     "offending line",
     "nf", b" " * 1000000, "1 MB of spaces -> ~2 MB of stderr"),

    ("D10", "LOW", "UTF-8 BOM is not skipped", "nf", "\ufeffzero", None),
    ("D11", "LOW", "`_` is neither a binder nor a term; the diagnostic is 'expected an atom'",
     "nf", "let x : Nat = zero; _", None),
    ("D11b", "LOW", "`_` as a lambda binder", "nf", "let f : (A : U) -> A -> A = \\A _ . A; f", None),
    ("D13", "LOW", "'more patterns than arrows' message does not say the expected count",
     "nf", "let f : Nat -> Nat -> Nat = \\p0 p1 p2 . p0; f", None),
    ("D13b", "OK", "control: 3 patterns with a 3-arrow type is accepted", "nf",
     "let f : (p0 p1 p2 : Nat) -> Nat = \\p0 p1 p2 . p0; f", None),
]


def main():
    lines = []
    lines.append("dpm-nbe black-box findings - automated reproduction")
    lines.append("binary: %s" % BIN)
    lines.append("sha256: %s" % subprocess.run(
        ["powershell", "-NoProfile", "-Command",
         "(Get-FileHash '%s' -Algorithm SHA256).Hash" % BIN],
        stdout=subprocess.PIPE).stdout.decode().strip())
    lines.append("=" * 100)
    for fid, sev, title, mode, prog, note in FINDINGS:
        rc, out, err, to = run(prog, mode, timeout=60)
        lines.append("")
        lines.append("[%s] %s  --  %s" % (fid, sev, title))
        if note:
            lines.append("    note      : %s" % note)
        p = prog if isinstance(prog, str) else repr(prog)
        pshow = p if len(p) <= 200 else p[:200] + "...<%d chars total>" % len(p)
        lines.append("    command   : printf %s | dpm-nbe %s" % (repr(pshow), mode))
        lines.append("    exit code : %s" % (rc if not to else "TIMEOUT/killed"))
        if rc is not None and rc not in (0,):
            lines.append("    exit hex  : 0x%08X" % (rc & 0xFFFFFFFF))
        lines.append("    stdout    : %s" % (repr(cap(out)) if out else "(empty)"))
        lines.append("    stderr    : %s" % (repr(cap(err)) if err else "(empty)"))
        print("[%s] rc=%s %s" % (fid, rc if not to else "TIMEOUT", title[:70]))
    # ---- multi-step: the succ round trip
    lines.append("")
    lines.append("[D3-full] round trip of the printed normal form (2 steps)")
    rc, out, err, _ = run("plus (suc zero) (suc zero)", "nf")
    printed = out.decode("utf-8", "replace")
    nf = printed.split("\n  :\n")[0]
    lines.append("    step 1    : plus (suc zero) (suc zero)  ->  exit %s" % rc)
    lines.append("                stdout = %r" % printed)
    rc2, out2, err2, _ = run(nf, "nf")
    lines.append("    step 2    : %r  ->  exit %s" % (nf, rc2))
    lines.append("                stderr = %r" % cap(err2, 200))
    with open(os.path.join(HERE, os.environ.get("DPM_OUT", "FINDINGS.txt")), "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")
    print("wrote", os.environ.get("DPM_OUT", "FINDINGS.txt"))


if __name__ == "__main__":
    main()
