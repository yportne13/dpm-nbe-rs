#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Differential / round-trip checks, round 2.

Two round-trip notions:
  RAW  : feed the printed normal form back verbatim -> tests printer/parser agreement.
  CANON: rescue only the known `succ`/`suc` printer-typo (word-boundary), and wrap
         the term in `let v : <printed type> = <printed nf>; v` so that terms whose
         type cannot be *inferred* (bare refl / bare lambda) are still checked.
         Everything else must round-trip byte-identically.
Invariants:
  B  : type(P) == type line of nf(P)
  C  : nf(nf(P)) == nf(P)                                  (raw)
  C2 : nf(let v : ty = nf; v) == nf(P)                     (canonical)
  D  : type(let v : ty = nf; v) == ty
  E  : nf(ty) is well-typed and its normal form is ty itself
"""
import json
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
LIVE = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"
PINNED = os.path.join(HERE, "bin", "pinned-33052DDC.exe")
BIN = os.environ.get("DPM_BIN") or (PINNED if os.path.exists(PINNED) else LIVE)
SEP = "\n  :\n"


def run(text, mode="nf", timeout=30):
    data = text.encode("utf-8") if isinstance(text, str) else text
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return "TIMEOUT", "", ""


def canon(s):
    return re.sub(r"\bsucc\b", "suc", s)


def split_nf(out):
    if not out.endswith("\n"):
        return None, None
    body = out[:-1]
    if SEP not in body:
        return None, None
    return body.split(SEP, 1)


def load_seeds():
    d = os.path.join(HERE, "corpus", "valid")
    seeds = []
    for fn in sorted(os.listdir(d)):
        if fn.endswith(".txt"):
            seeds.append((fn[:-4], open(os.path.join(d, fn), encoding="utf-8").read()))
    return seeds


def main():
    seeds = load_seeds()
    findings = []
    rows = []
    raw_fail_kind = {}
    for name, src in seeds:
        rec = {"name": name, "src": src.strip()}
        rc, out, err = run(src, "nf")
        rec["rc"] = rc
        if rc != 0:
            findings.append(("SEED-NOT-WELLTYPED", name, str(rc), err.strip().split("\n")[-1]))
            rows.append(rec)
            continue
        nf, ty = split_nf(out)
        rec["nf"] = nf
        rec["type"] = ty
        if nf is None:
            findings.append(("NO-SEPARATOR", name, "0", repr(out[:100])))
            rows.append(rec)
            continue

        rc_t, out_t, err_t = run(src, "type")
        rec["type_mode"] = out_t
        if rc_t != 0 or out_t != ty + "\n":
            findings.append(("B-TYPE-MODE-MISMATCH", name, str(rc_t),
                             "type=%r nf-line=%r" % (out_t, ty + "\n")))

        # --- RAW round trip
        rc2, out2, err2 = run(nf, "nf")
        rec["raw_rc"] = rc2
        if rc2 != 0 or out2 != out:
            last = err2.strip().split("\n")[-1] if err2 else "different output"
            kind = "succ-keyword" if "succ" in err2 else (
                "uninferable-top" if ("infer" in err2) else "other")
            raw_fail_kind[kind] = raw_fail_kind.get(kind, 0) + 1
            rec["raw_fail"] = kind + " | " + last
        else:
            rec["raw_fail"] = None

        # --- CANONICAL round trip
        prog = "let v : %s = %s; v" % (canon(ty), canon(nf))
        rc3, out3, err3 = run(prog, "nf")
        rec["canon_rc"] = rc3
        if rc3 != 0 or out3 != out:
            findings.append(("C2-CANON-ROUNDTRIP-FAIL", name, str(rc3),
                             "prog=%r\n      got rc=%s out=%r err=%r" % (
                                 prog[:200], rc3, out3[:200], err3.strip()[-200:])))
        # --- type of the canonical program
        rc4, out4, err4 = run(prog, "type")
        if rc4 != 0 or out4 != ty + "\n":
            findings.append(("D-TYPE-OF-CANON", name, str(rc4),
                             "got=%r want=%r err=%r" % (out4, ty, err4.strip()[-150:])))

        # --- the printed type itself must be a well-typed, normal term
        rc5, out5, err5 = run(canon(ty), "nf")
        if rc5 != 0:
            findings.append(("E-TYPE-NOT-WELLTYPED", name, str(rc5), err5.strip()[-200:]))
        else:
            tnf, tty = split_nf(out5)
            if tnf != canon(ty):
                findings.append(("E-TYPE-NOT-NORMAL", name, "0",
                                 "nf(type)=%r type=%r" % (tnf, canon(ty))))
            if tty != "U":
                findings.append(("E-TYPE-OF-TYPE", name, "0", "type of type = %r" % tty))
        rows.append(rec)

    summary = {
        "seeds": len(seeds),
        "raw_roundtrip_failures": sum(1 for r in rows if r.get("raw_fail")),
        "raw_fail_kinds": raw_fail_kind,
        "findings": len(findings),
    }
    with open(os.path.join(HERE, "diffcheck2.json"), "w", encoding="utf-8") as fh:
        json.dump({"summary": summary, "rows": rows, "findings": findings}, fh,
                  ensure_ascii=False, indent=1)
    print(json.dumps(summary, indent=1, ensure_ascii=False))
    print()
    for f in findings:
        print("!! %s  %s  rc=%s" % (f[0], f[1], f[2]))
        print("   ", f[3][:600].replace("\n", "\n    "))
    print()
    print("--- raw round-trip failures by kind ---")
    for r in rows:
        if r.get("raw_fail"):
            print("  %-28s %s | %s" % (r["name"], r["raw_fail"], r["nf"][:60]))


if __name__ == "__main__":
    main()
