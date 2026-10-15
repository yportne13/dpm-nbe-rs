#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Show tail/length of stored stdout/stderr for given probe ids."""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
for pid in sys.argv[1:]:
    for ext in ("err", "out"):
        p = os.path.join(HERE, "logs", "%s.%s" % (pid, ext))
        if not os.path.exists(p):
            continue
        b = open(p, "rb").read()
        if not b:
            continue
        print("--- %s.%s len=%d" % (pid, ext, len(b)))
        if len(b) <= 500:
            print(b.decode("utf-8", "replace"))
        else:
            print("HEAD:", b[:200].decode("utf-8", "replace").replace("\n", "\\n"))
            print("TAIL:", b[-300:].decode("utf-8", "replace").replace("\n", "\\n"))
