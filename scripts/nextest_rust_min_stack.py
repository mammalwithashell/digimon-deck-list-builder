#!/usr/bin/env python3
from __future__ import annotations

import os
import subprocess
import sys


def main() -> int:
    if len(sys.argv) < 2:
        print("nextest rust-min-stack wrapper requires a test binary command", file=sys.stderr)
        return 2
    os.environ.setdefault("RUST_MIN_STACK", "268435456")
    return subprocess.call(sys.argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
