#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Summarise results.jsonl."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
rows = [json.loads(l) for l in open(os.path.join(HERE, "results.jsonl"), encoding="utf-8") if l.strip()]
man = {p["id"]: p for p in json.load(open(os.path.join(HERE, "manifest.json"), encoding="utf-8"))}

WANT = sys.argv[1:] if len(sys.argv) > 1 else None

cats = {}
for r in rows:
    cats.setdefault(r["category"], []).append(r)

print("total rows:", len(rows))
for c, rs in sorted(cats.items()):
    rc = {}
    for r in rs:
        rc[r["returncode"]] = rc.get(r["returncode"], 0) + 1
    print("  %-10s %4d  rc=%s  max_time=%.0fms max_mem=%.1fMB" % (
        c, len(rs), rc, max(x["elapsed_ms"] for x in rs), max(x["peak_ws_mb"] for x in rs)))

print()
print("=== anomalous (rc not 0/1, or timeout) ===")
for r in rows:
    if r["returncode"] not in (0, 1) or r["timed_out"]:
        print("%-6s %-9s rc=%-12s %s" % (r["id"], r["category"], r["returncode"], r["note"]))
        print("   stderr:", r["stderr"][:300].replace("\n", "\\n"))

if WANT:
    print()
    for r in rows:
        if any(r["id"] == w or r["id"].startswith(w) for w in WANT):
            print("=" * 100)
            print("%s  cat=%s args=%s rc=%s %.1fms %.1fMB" % (
                r["id"], r["category"], r["args"], r["returncode"], r["elapsed_ms"], r["peak_ws_mb"]))
            print("  note:", r["note"], "| stdin_len:", r["stdin_len"])
            print("  STDOUT:", repr(r["stdout"][:1200]))
            print("  STDERR:", repr(r["stderr"][:1200]))
