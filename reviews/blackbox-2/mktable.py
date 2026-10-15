#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Produce PROBE_TABLE.md: every harness probe with command, exit code,
observed output and a verdict.  Raw stdout/stderr live in logs/<id>.{out,err}."""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
rows = [json.loads(l) for l in open(os.path.join(HERE, "results.jsonl"), encoding="utf-8") if l.strip()]
man = {p["id"]: p for p in json.load(open(os.path.join(HERE, "manifest.json"), encoding="utf-8"))}


def esc(s, n=110):
    s = s.replace("\\", "\\\\").replace("|", "\\|").replace("\n", "\\n").replace("\r", "\\r")
    s = s.replace("\t", "\\t").replace("\x00", "\\0").replace("`", "'")
    return s if len(s) <= n else s[:n] + "\u2026"


def stdout_repr(r):
    return esc(r["stdout"][:110])


def stderr_last(r):
    e = r["stderr"].strip("\n")
    if not e:
        return ""
    return esc(e.split("\n")[-1][:110] if r["returncode"] != 101 else "thread 'dpm-nbe' panicked ... " + e.split("\n")[-1][:60])


def verdict(r):
    rc = r["returncode"]
    if r["timed_out"]:
        return "**TIMEOUT / stack overflow**"
    if rc == 0:
        return "ok"
    if rc == 1:
        return "clean error (exit 1)"
    if rc == 2:
        return "exit 2 (stdin not UTF-8)"
    if rc == 101:
        return "**RUST PANIC (exit 101)**"
    if rc is not None and rc & 0xFFFFFFFF == 0xC00000FD:
        return "**STATUS_STACK_OVERFLOW (0xC00000FD)**"
    return "**unexpected exit %s**" % r.get("rc_hex")


def stdin_repr(r):
    p = man[r["id"]]["stdin_file"]
    data = open(os.path.join(HERE, p), "rb").read()
    if len(data) > 60:
        return esc(data[:60].decode("utf-8", "replace")) + "\u2026(%d B)" % len(data)
    return esc(data.decode("utf-8", "replace"))


out = []
out.append("# Probe table (machine generated)\n")
out.append("Binary under test: `bin/pinned-33052DDC.exe` "
           "(SHA256 33052DDC9B0E113E6BFC57F04822F932CE14A8574B1C63A0EBD1303415D34854)\n")
out.append("Probes: **%d**. Raw bytes for every probe: `logs/<id>.out` / `logs/<id>.err`.\n" % len(rows))
cats = {}
for r in rows:
    cats.setdefault(r["category"], []).append(r)
for cat in ("parser", "depth", "blowup", "semantic", "cli", "regress"):
    rs = cats.get(cat, [])
    out.append("\n## %s (%d probes)\n" % (cat, len(rs)))
    out.append("| id | command | stdin (truncated) | exit | stdout | stderr last line | verdict | note |")
    out.append("|---|---|---|---|---|---|---|---|")
    for r in rs:
        out.append("| %s | `dpm-nbe %s` | `%s` | %s | `%s` | `%s` | %s | %s |" % (
            r["id"], " ".join(r["args"]) or "(no args)", stdin_repr(r),
            r["returncode"] if not r["timed_out"] else "killed",
            stdout_repr(r), stderr_last(r), verdict(r), esc(r["note"], 70)))
out.append("")
with open(os.path.join(HERE, "PROBE_TABLE.md"), "w", encoding="utf-8") as fh:
    fh.write("\n".join(out))
print("wrote PROBE_TABLE.md with", len(rows), "rows")
vc = {}
for r in rows:
    vc[verdict(r)] = vc.get(verdict(r), 0) + 1
for k, v in sorted(vc.items(), key=lambda x: -x[1]):
    print("  %-40s %d" % (k, v))
