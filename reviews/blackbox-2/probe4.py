#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 4: quantify the stack-overflow / hang and the quadratic scaling."""
import ctypes
import ctypes.wintypes as wt
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("DPM_BIN") or os.path.join(HERE, "bin", "pinned-33052DDC.exe")

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
k32.OpenProcess.restype = wt.HANDLE
k32.OpenProcess.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]


class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", wt.DWORD), ("dwHighDateTime", wt.DWORD)]


def cpu_ms(handle):
    c, e, kt, ut = FILETIME(), FILETIME(), FILETIME(), FILETIME()
    if k32.GetProcessTimes(handle, ctypes.byref(c), ctypes.byref(e),
                           ctypes.byref(kt), ctypes.byref(ut)):
        t = (kt.dwHighDateTime << 32 | kt.dwLowDateTime) + \
            (ut.dwHighDateTime << 32 | ut.dwLowDateTime)
        return t / 10000.0
    return -1


def run_measured(text, timeout, tag):
    data = text.encode("utf-8")
    outf = os.path.join(HERE, "logs", "probe4_%s.out" % tag)
    errf = os.path.join(HERE, "logs", "probe4_%s.err" % tag)
    with open(outf, "wb") as o, open(errf, "wb") as e:
        p = subprocess.Popen([BIN, "nf"], stdin=subprocess.PIPE, stdout=o, stderr=e)
        h = k32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, p.pid)
        t0 = time.perf_counter()
        # write stdin in a thread-free way: the input is tiny
        try:
            p.stdin.write(data)
            p.stdin.close()
        except Exception as ex:
            print("   stdin write failed:", ex)
        rc = None
        peak_cpu = 0.0
        exited_at = None
        while time.perf_counter() - t0 < timeout:
            rc = p.poll()
            if rc is not None:
                exited_at = time.perf_counter() - t0
                break
            c = cpu_ms(h)
            if c > peak_cpu:
                peak_cpu = c
            time.sleep(0.05)
        if rc is None:
            rc = p.poll()
        if rc is None:
            print("   >>> STILL RUNNING after %.0fs (CPU %.1fs) -> KILLING" % (
                timeout, peak_cpu / 1000.0))
            p.kill()
            p.wait()
        else:
            print("   >>> exited after %.2fs rc=%s cpu=%.1fs" % (
                exited_at if exited_at else -1, rc, peak_cpu / 1000.0))
    osz = os.path.getsize(outf)
    esz = os.path.getsize(errf)
    print("   stdout=%d bytes stderr=%d bytes" % (osz, esz))
    if esz:
        print("   stderr:", open(errf, "rb").read()[:200])
    return rc


print("### stack-overflow / hang threshold for doubling chain")
for k in (14, 15, 16, 17, 18, 20):
    body = "suc zero"
    for _ in range(k):
        body = "d (" + body + ")"
    src = "let d : Nat -> Nat = \\n . plus n n; " + body
    print("k=%d input=%d bytes" % (k, len(src)))
    run_measured(src, 90.0, "k%d" % k)

print()
print("### a different deep-value shape: nested lets + plus")
src = "let a : Nat = suc zero; "
for i in range(12):
    src += "let b%d : Nat = plus a a; let a : Nat = b%d; " % (i, i)
src += "a"
print("nested-let doubling, input=%d bytes" % len(src))
run_measured(src, 60.0, "nestedlet")
