#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Compare results.jsonl (pinned build 33052DDC) with results_live.jsonl."""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))


def load(fn):
    p = os.path.join(HERE, fn)
    if not os.path.exists(p):
        return {}
    return {json.loads(l)["id"]: json.loads(l)
            for l in open(p, encoding="utf-8") if l.strip()}


a = load("results.jsonl")
b = load("results_live.jsonl")
man = {p["id"]: p for p in json.load(open(os.path.join(HERE, "manifest.json"), encoding="utf-8"))}
print("pinned probes: %d   live probes: %d" % (len(a), len(b)))
chg = []
for pid in sorted(set(a) & set(b), key=lambda x: (x[0] != "P", int("".join(c for c in x if c.isdigit()) or 0))):
    ra, rb = a[pid], b[pid]
    if ra["returncode"] != rb["returncode"] or ra["timed_out"] != rb["timed_out"]:
        chg.append((pid, man[pid]["note"][:60], ra["returncode"], rb["returncode"]))
print("\nverdict changes pinned -> live: %d" % len(chg))
for pid, note, x, y in chg:
    print("  %-6s rc %-12s -> %-12s  %s" % (pid, x, y, note))

# stdout changes for probes that stayed at rc 0
print("\nstdout changes among rc==0 probes:")
n = 0
for pid in sorted(set(a) & set(b)):
    ra, rb = a[pid], b[pid]
    if ra["returncode"] == 0 and rb["returncode"] == 0 and ra["stdout"] != rb["stdout"]:
        n += 1
        if n <= 25:
            print("  %-6s %-45s %r -> %r" % (pid, man[pid]["note"][:42],
                                             ra["stdout"][:40], rb["stdout"][:40]))
print("  total:", n)
