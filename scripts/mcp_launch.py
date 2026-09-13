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

This resolves the binary where cargo actually put it — `CARGO_TARGET_DIR` when
set, else the per-worktree dir derived from `CARGO_TARGET_BASE` exactly as
`~/.bashrc` derives it (the app's MCP environment has the base but not the
derived dir), else `<repo>/target` — prefers `release` over `debug`, and
**execs it directly**. No build happens here on purpose: a missing binary exits non-zero
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


def derived_target_dir() -> Path:
    """The per-worktree target dir, derived exactly as `~/.bashrc` derives it.

    `CARGO_TARGET_DIR` is NOT a User environment variable here: `~/.bashrc`
    computes it per worktree (CLAUDE.md rule 31), so it exists in a bash shell
    and is ABSENT in the environment the desktop app hands its MCP servers. The
    launcher used to fall straight back to `<repo>/target` in that case — a
    directory that never exists under the isolation scheme — so every Rust MCP
    server failed to connect with "no built binary" while the binary sat in
    `D:/cargo-target/<worktree>`. A health check run from a bash shell passed,
    because that shell had the variable; only the app's launch reproduced it.

    `CARGO_TARGET_BASE` IS a User variable, so it is visible to the app. Mirror
    the bashrc rule: `<base>/<worktree-name>` inside `.claude/worktrees/`, else
    `<base>/<repo basename>`.
    """
    base = Path(os.environ.get("CARGO_TARGET_BASE") or "D:/cargo-target")
    root = repo_root()
    parts = root.parts
    for i in range(len(parts) - 2):
        if parts[i] == ".claude" and parts[i + 1] == "worktrees":
            return base / parts[i + 2]
    return base / root.name


def target_dirs() -> list[Path]:
    """Where cargo may have written build output, most-authoritative first.

    An explicit `CARGO_TARGET_DIR` wins outright (a bash shell, CI, or a pinned
    build). Otherwise the per-worktree derived dir, then `<repo>/target` for a
    machine that does not use the isolation scheme at all.
    """
    env = os.environ.get("CARGO_TARGET_DIR")
    dirs = [Path(env)] if env else []
    for d in (derived_target_dir(), repo_root() / "target"):
        if d not in dirs:
            dirs.append(d)
    return dirs


def candidates(binary: str) -> list[Path]:
    """Plausible locations, most-preferred first.

    Directories in `target_dirs()` order; within each, `release` wins over
    `debug`: if someone has built an optimized server they almost certainly
    want it, and a stale debug build alongside it would otherwise shadow it
    silently.
    """
    exe = ".exe" if os.name == "nt" else ""
    return [
        root / profile / f"{binary}{exe}"
        for root in target_dirs()
        for profile in ("release", "debug")
    ]


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
        f"  (CARGO_TARGET_DIR={os.environ.get('CARGO_TARGET_DIR') or '<unset>'}, "
        f"CARGO_TARGET_BASE={os.environ.get('CARGO_TARGET_BASE') or '<unset>'}; "
        f"build from this worktree so cargo writes to the first dir above)",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
