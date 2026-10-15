#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Corpus-wide cross checks:
  1. for every rc==0 `nf` probe: type mode output == the type line of nf output
  2. stream discipline: rc==0 => stderr empty; rc!=0 => stdout empty
  3. every rc==0 `nf` probe's printed nf is re-parseable after the succ->suc rescue
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


def run(data, mode, timeout=60):
    try:
        p = subprocess.run([BIN, mode], input=data, stdout=subprocess.PIPE,
                           stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return "TIMEOUT", b"", b""


def main():
    rows = [json.loads(l) for l in open(os.path.join(HERE, "results.jsonl"), encoding="utf-8") if l.strip()]
    man = {p["id"]: p for p in json.load(open(os.path.join(HERE, "manifest.json"), encoding="utf-8"))}
    checks = {"type_mismatch": [], "stdout_polluted": [], "stderr_dirty": [],
              "raw_roundtrip_fail": [], "canon_roundtrip_fail": [], "checked": 0,
              "roundtrip_checked": 0}
    for r in rows:
        if r["returncode"] == 0 and not r["timed_out"]:
            if r["stderr"]:
                checks["stderr_dirty"].append((r["id"], r["stderr"][:120]))
        if r["returncode"] not in (0, None) and not r["timed_out"]:
            if r["stdout"]:
                checks["stdout_polluted"].append((r["id"], r["stdout"][:120]))
        if r["returncode"] != 0 or r["timed_out"] or r["args"] != ["nf"]:
            continue
        out = r["stdout"]
        if not out.endswith("\n") or SEP not in out[:-1]:
            continue
        nf, ty = out[:-1].split(SEP, 1)
        checks["checked"] += 1
        data = open(os.path.join(HERE, man[r["id"]]["stdin_file"]), "rb").read()
        rc, o, e = run(data, "type", timeout=60)
        if rc != 0 or o.decode("utf-8", "replace") != ty + "\n":
            checks["type_mismatch"].append((r["id"], r["note"][:50],
                                            o.decode("utf-8", "replace")[:80], ty[:80]))
        # round trip of the printed nf
        if len(nf) < 20000:
            checks["roundtrip_checked"] += 1
            rc2, o2, e2 = run(nf.encode("utf-8"), "nf", timeout=60)
            if rc2 != 0 or o2.decode("utf-8", "replace") != out:
                if b"succ" in e2:
                    kind = "succ"
                elif b"infer" in e2:
                    kind = "infer"
                else:
                    kind = e2.decode("utf-8", "replace").strip().split("\n")[-1][:60]
                checks["raw_roundtrip_fail"].append((r["id"], r["note"][:40], kind))
            cnf = re.sub(r"\bsucc\b", "suc", nf)
            cty = re.sub(r"\bsucc\b", "suc", ty)
            prog = "let v : %s = %s; v" % (cty, cnf)
            rc3, o3, e3 = run(prog.encode("utf-8"), "nf", timeout=60)
            if rc3 != 0 or o3.decode("utf-8", "replace") != out:
                checks["canon_roundtrip_fail"].append(
                    (r["id"], r["note"][:40], rc3,
                     e3.decode("utf-8", "replace").strip().split("\n")[-1][:80]))

    print(json.dumps({k: (len(v) if isinstance(v, list) else v) for k, v in checks.items()}, indent=1))
    for k in ("type_mismatch", "stdout_polluted", "stderr_dirty", "canon_roundtrip_fail"):
        for item in checks[k][:20]:
            print("!! %-22s %s" % (k, item))
    print("--- raw round-trip failures (first 30 of %d) ---" % len(checks["raw_roundtrip_fail"]))
    kinds = {}
    for i in checks["raw_roundtrip_fail"]:
        kinds[i[2]] = kinds.get(i[2], 0) + 1
    print("   by cause:", kinds)
    for item in checks["raw_roundtrip_fail"][:30]:
        print("   %-8s %-40s %s" % item)
    with open(os.path.join(HERE, "crosscheck.json"), "w", encoding="utf-8") as fh:
        json.dump(checks, fh, ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main()
