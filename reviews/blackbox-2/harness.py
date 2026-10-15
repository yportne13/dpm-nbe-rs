#!/usr/bin/env python3
"""Black-box probe harness for dpm-nbe.exe.

Runs a manifest of probes, each with a hard timeout, capturing exit code,
raw stdout/stderr, wall time and sampled peak working set.

Manifest format (JSON list):
  [ {"id": "P001", "category": "parser", "args": ["nf"],
     "stdin_file": "corpus/P001.txt",     # or "stdin_text" or "stdin_b64"
     "timeout": 10.0, "note": "..."} , ... ]

Usage:
  python harness.py --manifest manifest.json --results results.jsonl --logdir logs
"""
import argparse
import base64
import ctypes
import ctypes.wintypes as wt
import json
import os
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
LIVE = r"F:\projects\Haskell\dpm-nbe\rust\target\release\dpm-nbe.exe"
PINNED = os.path.join(HERE, "bin", "pinned-33052DDC.exe")
BIN = os.environ.get("DPM_BIN") or (PINNED if os.path.exists(PINNED) else LIVE)

# ---------------------------------------------------------------- memory probe
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
PROCESS_VM_READ = 0x0010


class PMC(ctypes.Structure):
    _fields_ = [
        ("cb", wt.DWORD),
        ("PageFaultCount", wt.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
psapi = ctypes.WinDLL("psapi", use_last_error=True)
kernel32.OpenProcess.restype = wt.HANDLE
kernel32.OpenProcess.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
kernel32.CloseHandle.argtypes = [wt.HANDLE]
psapi.GetProcessMemoryInfo.argtypes = [wt.HANDLE, ctypes.POINTER(PMC), wt.DWORD]
psapi.GetProcessMemoryInfo.restype = wt.BOOL


def sample_mem(handle):
    if not handle:
        return 0
    m = PMC()
    m.cb = ctypes.sizeof(PMC)
    if psapi.GetProcessMemoryInfo(handle, ctypes.byref(m), m.cb):
        return int(m.WorkingSetSize)
    return 0


# ---------------------------------------------------------------------- runner
def run_probe(p, logdir):
    rid = p["id"]
    args = p.get("args", ["nf"])
    timeout = float(p.get("timeout", 10.0))

    if "stdin_b64" in p:
        data = base64.b64decode(p["stdin_b64"])
    elif "stdin_text" in p:
        data = p["stdin_text"].encode("utf-8")
    elif "stdin_file" in p:
        with open(os.path.join(HERE, p["stdin_file"]), "rb") as fh:
            data = fh.read()
    else:
        data = b""

    cmd = [BIN] + list(args)
    t0 = time.perf_counter()
    try:
        proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=HERE,
        )
    except OSError as e:
        return {
            "id": rid, "category": p.get("category", "?"), "args": args,
            "stdin_len": len(data), "note": p.get("note", ""),
            "launch_error": str(e), "elapsed_ms": 0.0, "returncode": None,
            "timed_out": False, "peak_ws_mb": 0.0, "stdout": "", "stderr": "",
        }

    handle = kernel32.OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, False, proc.pid
    )
    out_chunks, err_chunks = [], []

    def reader(stream, sink):
        try:
            while True:
                b = stream.read(65536)
                if not b:
                    break
                sink.append(b)
        except Exception:
            pass
        finally:
            try:
                stream.close()
            except Exception:
                pass

    def writer():
        try:
            if data:
                proc.stdin.write(data)
            proc.stdin.close()
        except Exception:
            pass

    t_out = threading.Thread(target=reader, args=(proc.stdout, out_chunks), daemon=True)
    t_err = threading.Thread(target=reader, args=(proc.stderr, err_chunks), daemon=True)
    t_in = threading.Thread(target=writer, daemon=True)
    t_out.start(); t_err.start(); t_in.start()

    peak = 0
    timed_out = False
    while True:
        rc = proc.poll()
        if rc is not None:
            break
        mem = sample_mem(handle)
        if mem > peak:
            peak = mem
        if time.perf_counter() - t0 > timeout:
            timed_out = True
            try:
                proc.kill()
            except Exception:
                pass
            try:
                proc.wait(timeout=10)
            except Exception:
                pass
            break
        time.sleep(0.01)

    t_out.join(timeout=10)
    t_err.join(timeout=10)
    t_in.join(timeout=5)
    if handle:
        kernel32.CloseHandle(handle)
    elapsed = (time.perf_counter() - t0) * 1000.0

    out_b = b"".join(out_chunks)
    err_b = b"".join(err_chunks)
    rc = proc.returncode

    os.makedirs(logdir, exist_ok=True)
    with open(os.path.join(logdir, rid + ".out"), "wb") as fh:
        fh.write(out_b)
    with open(os.path.join(logdir, rid + ".err"), "wb") as fh:
        fh.write(err_b)

    def dec(b):
        return b.decode("utf-8", errors="replace")

    return {
        "id": rid,
        "category": p.get("category", "?"),
        "args": args,
        "stdin_len": len(data),
        "note": p.get("note", ""),
        "returncode": rc,
        "rc_hex": ("0x%08X" % (rc & 0xFFFFFFFF)) if rc is not None and rc >= 0 else None,
        "timed_out": timed_out,
        "elapsed_ms": round(elapsed, 1),
        "peak_ws_mb": round(peak / (1024.0 * 1024.0), 2),
        "stdout": dec(out_b),
        "stderr": dec(err_b),
        "stdout_len": len(out_b),
        "stderr_len": len(err_b),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", default=os.path.join(HERE, "manifest.json"))
    ap.add_argument("--results", default=os.path.join(HERE, "results.jsonl"))
    ap.add_argument("--logdir", default=os.path.join(HERE, "logs"))
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--only", default=None, help="comma separated id prefixes")
    args = ap.parse_args()

    with open(args.manifest, "r", encoding="utf-8") as fh:
        manifest = json.load(fh)

    only = args.only.split(",") if args.only else None
    done = set()
    if args.resume and os.path.exists(args.results):
        with open(args.results, "r", encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if line:
                    try:
                        done.add(json.loads(line)["id"])
                    except Exception:
                        pass

    mode = "a" if args.resume else "w"
    with open(args.results, mode, encoding="utf-8") as rf:
        for p in manifest:
            if only and not any(p["id"].startswith(o) for o in only):
                continue
            if p["id"] in done:
                continue
            res = run_probe(p, args.logdir)
            rf.write(json.dumps(res, ensure_ascii=False) + "\n")
            rf.flush()
            flag = ""
            if res.get("timed_out"):
                flag = " TIMEOUT"
            elif res.get("returncode") not in (0, 1):
                flag = " RC=%s" % res.get("rc_hex")
            print("%-8s %-14s rc=%-12s %7.1fms %6.1fMB%s" % (
                res["id"], res["category"], res.get("returncode"),
                res["elapsed_ms"], res["peak_ws_mb"], flag), flush=True)


if __name__ == "__main__":
    main()
