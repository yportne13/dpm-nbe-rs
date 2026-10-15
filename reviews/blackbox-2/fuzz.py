#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Grammar + mutation fuzzer for dpm-nbe.exe (black box).

Finds: Rust panics (rc=101), aborts (0xC0000409 stack overflow), access
violations (0xC0000005), timeouts, and any rc outside {0,1}.
"""
import argparse
import hashlib
import json
import os
import random
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
LIVE = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"
PINNED = os.path.join(HERE, "bin", "pinned-33052DDC.exe")
BIN = os.environ.get("DPM_BIN") or (PINNED if os.path.exists(PINNED) else LIVE)

TOKENS = [
    "zero", "suc", "plus", "Id", "Nat", "U", "refl", "let", "in", "x", "y", "f",
    ":", ";", "=", ".", ",", "->", "\u2192", "\\", "\u03bb", "(", ")", "(!)", "_",
    "--", "{-", "-}", "\n", " ", "\t", "->", "U", "suc", "Nat", "0", "aa", "\u03b1",
    "'", "?", "#", "%", "@", "$", "|", "&", "~", "!", "[]", "{}", '"', "`",
]

SEEDS = []
for d in (os.path.join(HERE, "corpus", "valid"), os.path.join(HERE, "seeds")):
    if os.path.isdir(d):
        for fn in sorted(os.listdir(d)):
            if fn.endswith(".txt"):
                SEEDS.append(open(os.path.join(d, fn), "rb").read())

VALID = [
    b"zero", b"plus (suc zero) (suc zero)", b"Nat -> Nat",
    b"let f : Nat -> Nat = \\n . n; f", b"(A : U) -> A -> A", b"Id Nat zero zero",
    b"let x : Nat = zero; x", b"U", b"let C : (A : U) -> Id Nat (suc zero) zero -> A = \\A (!); U",
    b"let trans : (A : U) (a b c : A) -> Id A a b -> Id A b c -> Id A a c = \\A a b c refl refl . refl; trans",
]


# --------------------------------------------------------------- generators
def gen_tokens(rng):
    n = rng.randint(1, 40)
    parts = []
    for _ in range(n):
        parts.append(rng.choice(TOKENS))
        if rng.random() < 0.5:
            parts.append(" ")
    return "".join(parts).encode("utf-8")


def gen_struct(rng, depth=0):
    """Random syntactically plausible term."""
    if depth > 4 or rng.random() < 0.3:
        return rng.choice(["zero", "U", "Nat", "x", "y", "f", "refl", "suc zero",
                           "plus zero zero", "Id Nat zero zero", "aa", "\u03b1"])
    k = rng.randint(0, 6)
    if k == 0:
        return "(" + gen_struct(rng, depth + 1) + ")"
    if k == 1:
        return "suc (" + gen_struct(rng, depth + 1) + ")"
    if k == 2:
        return "plus (" + gen_struct(rng, depth + 1) + ") (" + gen_struct(rng, depth + 1) + ")"
    if k == 3:
        return "(" + gen_struct(rng, depth + 1) + ") -> " + gen_struct(rng, depth + 1)
    if k == 4:
        return "(A : U) -> " + gen_struct(rng, depth + 1)
    if k == 5:
        return "\\A x . " + gen_struct(rng, depth + 1)
    return gen_struct(rng, depth + 1) + " " + gen_struct(rng, depth + 1)


def gen_program(rng):
    """let-prefixed program."""
    nlets = rng.randint(0, 4)
    parts = []
    for i in range(nlets):
        nm = rng.choice(["x", "y", "f", "aa", "\u03b1", "x1"])
        ty = rng.choice(["Nat", "U", "Nat -> Nat", "(A : U) -> A -> A",
                         "Id Nat zero zero", "Id Nat (suc zero) zero"])
        val = rng.choice(["zero", "suc zero", "\\n . n", "\\A x . x", "refl",
                          "plus zero zero", "(!)", "U", "Nat"])
        parts.append("let %s : %s = %s; " % (nm, ty, val))
    parts.append(gen_struct(rng))
    return "".join(parts).encode("utf-8")


def gen_mutation(rng):
    b = bytearray(rng.choice(SEEDS + VALID))
    for _ in range(rng.randint(1, 6)):
        if not b:
            b = bytearray(b"zero")
        op = rng.randint(0, 7)
        i = rng.randrange(len(b))
        if op == 0:
            b[i] = rng.randrange(256)
        elif op == 1:
            b.insert(i, rng.randrange(256))
        elif op == 2:
            del b[i]
        elif op == 3:
            j = min(len(b), i + rng.randint(1, 12))
            b[i:i] = b[i:j]
        elif op == 4:
            j = min(len(b), i + rng.randint(1, 12))
            del b[i:j]
        elif op == 5:
            b[i:i] = rng.choice(
                [b"(", b")", b"\\", b".", b"(!)", b"->", b";", b"\x00", b"\xff",
                 b"\n", b"{-", b"-}", b"--"])
        elif op == 6:
            b = bytearray(bytes(b).replace(rng.choice([b"let", b"suc", b"Nat", b"zero"]),
                                           rng.choice([b"letx", b"Nat", b"refl", b"x"])))
        else:
            b[i:i] = rng.choice([b"\xef\xbb\xbf", b"\xc3\xa9", b"\xe2\x86\x92", b"\xce\xbb"])
    return bytes(b)


# ------------------------------------------------------------------ runner
def run_one(b, mode, timeout):
    try:
        p = subprocess.run([BIN, mode], input=b, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        rc, out, err = p.returncode, p.stdout, p.stderr
        to = False
    except subprocess.TimeoutExpired as e:
        rc, out, err, to = None, e.stdout or b"", e.stderr or b"", True
    return rc, out, err, to


def interesting(rc, out, err, to):
    if to:
        return "TIMEOUT"
    if rc == 2 and b"cannot read stdin" in err:
        return "STDIN-UTF8"
    if rc in (0, 1):
        if b"panicked" in err or b"RUST_BACKTRACE" in err:
            return "PANIC-QUIET"
        return None
    if rc == 101:
        return "PANIC"
    return "RC-%s" % (("0x%08X" % (rc & 0xFFFFFFFF)) if rc is not None and rc >= 0 else rc)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-n", type=int, default=4000)
    ap.add_argument("--timeout", type=float, default=5.0)
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--tag", default="fuzz")
    args = ap.parse_args()

    rng = random.Random(args.seed)
    crashdir = os.path.join(HERE, "fuzz", "crashes")
    os.makedirs(crashdir, exist_ok=True)

    cases = []
    for i in range(args.n):
        r = rng.random()
        if r < 0.30:
            b = gen_tokens(rng)
        elif r < 0.60:
            b = gen_program(rng)
        elif r < 0.90:
            b = gen_mutation(rng)
        else:
            b = gen_struct(rng).encode("utf-8")
        mode = rng.choice(["nf", "nf", "nf", "type"])
        cases.append((b, mode))

    results = []
    counts = {}
    findings = {}
    logf = open(os.path.join(HERE, "fuzz", "%s.jsonl" % args.tag), "w", encoding="utf-8")

    def work(item):
        b, mode = item
        rc, out, err, to = run_one(b, mode, args.timeout)
        kind = interesting(rc, out, err, to)
        return b, mode, rc, out, err, to, kind

    with ThreadPoolExecutor(max_workers=args.workers) as ex:
        for idx, (b, mode, rc, out, err, to, kind) in enumerate(ex.map(work, cases)):
            counts[kind or "clean"] = counts.get(kind or "clean", 0) + 1
            if kind:
                sig = kind + "|" + re.sub(rb"[0-9]+", b"N", err[:200]).decode("utf-8", "replace")
                if sig not in findings:
                    h = hashlib.sha256(b).hexdigest()[:16]
                    fn = os.path.join(crashdir, "%s-%s.txt" % (kind.replace("/", "_"), h))
                    with open(fn, "wb") as fh:
                        fh.write(b)
                    findings[sig] = {"kind": kind, "rc": rc, "mode": mode,
                                     "input": b[:400].decode("utf-8", "replace"),
                                     "input_len": len(b), "file": fn,
                                     "stderr": err[:400].decode("utf-8", "replace"),
                                     "stdout": out[:200].decode("utf-8", "replace"),
                                     "timed_out": to}
                    print("!! %-10s idx=%d rc=%s mode=%s len=%d -> %s" % (
                        kind, idx, rc, mode, len(b), fn), flush=True)
                    print("   in : %r" % b[:200], flush=True)
                    print("   err: %r" % err[:200], flush=True)
            logf.write(json.dumps({"i": idx, "mode": mode, "rc": rc, "to": to,
                                   "kind": kind, "len": len(b),
                                   "in": b[:200].decode("utf-8", "replace"),
                                   "err": err[:300].decode("utf-8", "replace")},
                                  ensure_ascii=False) + "\n")
            if idx % 500 == 0:
                logf.flush()
                print("... %d/%d %s" % (idx, args.n, counts), flush=True)
    logf.close()
    print("DONE n=%d counts=%s unique_findings=%d" % (args.n, counts, len(findings)))
    with open(os.path.join(HERE, "fuzz", "%s-findings.json" % args.tag), "w", encoding="utf-8") as fh:
        json.dump(findings, fh, ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main()
