#!/usr/bin/env python3
"""Launch a prebuilt Rust MCP server binary, wherever cargo actually put it.

`.mcp.json` used to reach these servers two ways, and both break here:

- `cargo run -p <crate> -- mcp` compiles inside the MCP handshake. A cold build
  of this workspace takes minutes; the client gives up after ~30 s and reports
  a connection timeout, which reads as "the server is broken" rather than "it
  was still compiling".
- A repo-relative `target/debug/<name>.exe` assumes cargo writes into the
  worktree. CLAUDE.md rule 31 moved build output to a per-worktree directory on
  another drive (`CARGO_TARGET_DIR`), so that path does not exist in a worktree
  at all.

This resolves the binary the same way cargo does — `CARGO_TARGET_DIR` when set,
otherwise `<repo>/target` — prefers `release` over `debug`, and **execs it
directly**. No build happens here on purpose: a missing binary exits non-zero
with the exact command to run, which the client surfaces immediately, instead
of stalling the handshake until it times out.

Usage (from `.mcp.json`, cwd = repo root):

    python scripts/mcp_launch.py <crate-binary-name> [args passed to the server]

e.g. `python scripts/mcp_launch.py dcgo-harness mcp`

Standard library only; no third-party imports, because this runs before
anything else in the session is known to be installed.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def repo_root() -> Path:
    """The repo (or worktree) root — this file lives in `<root>/scripts/`."""
    return Path(__file__).resolve().parent.parent


def target_dir() -> Path:
    """Where cargo writes build output, resolved as cargo itself would."""
    env = os.environ.get("CARGO_TARGET_DIR")
    if env:
        return Path(env)
    return repo_root() / "target"


def candidates(binary: str) -> list[Path]:
    """Plausible locations, most-preferred first.

    `release` wins over `debug`: if someone has built an optimized server they
    almost certainly want it, and a stale debug build alongside it would
    otherwise shadow it silently.
    """
    root = target_dir()
    exe = ".exe" if os.name == "nt" else ""
    return [root / profile / f"{binary}{exe}" for profile in ("release", "debug")]


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(
            "usage: mcp_launch.py <crate-binary-name> [server args...]",
            file=sys.stderr,
        )
        return 2

    binary, server_args = argv[1], argv[2:]
    tried = candidates(binary)
    for path in tried:
        if path.is_file():
            # Hand over stdio untouched: the MCP transport IS this process's
            # stdin/stdout, so the child must inherit them, and nothing may be
            # written to stdout here that is not JSON-RPC.
            completed = subprocess.run([str(path), *server_args])
            return completed.returncode

    print(
        f"mcp_launch: no built binary for {binary!r}.\n"
        f"  looked in: {', '.join(str(p) for p in tried)}\n"
        f"  build it once with:  cargo build -p {binary}\n"
        f"  (CARGO_TARGET_DIR is "
        f"{os.environ.get('CARGO_TARGET_DIR') or '<unset, using ./target>'})",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
