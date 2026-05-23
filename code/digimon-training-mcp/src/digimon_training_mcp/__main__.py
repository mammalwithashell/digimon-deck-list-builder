"""CLI entrypoint: ``python -m digimon_training_mcp``.

Parses ``--runs-dir`` / ``--models-dir`` flags, resolves the artifact roots
via ancestor-walk fallback, prints a one-line readiness banner to stderr
(mirroring ``digimon-engine-mcp``'s startup behavior), and starts the MCP
stdio server.
"""

from __future__ import annotations

import argparse
import asyncio
import logging
import sys
from pathlib import Path

from . import __version__
from .paths import resolve_models_dir, resolve_repo_root, resolve_runs_dir
from .server import ServerContext, run_stdio


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="digimon-training-mcp",
        description="Read-only MCP stdio server for inspecting Digimon RL training runs.",
    )
    parser.add_argument(
        "--runs-dir",
        type=Path,
        default=None,
        help="Override the runs/ directory (default: ancestor-walk from cwd).",
    )
    parser.add_argument(
        "--models-dir",
        type=Path,
        default=None,
        help="Override the models/ directory (default: ancestor-walk from cwd).",
    )
    parser.add_argument(
        "--log-level",
        default="WARNING",
        choices=["DEBUG", "INFO", "WARNING", "ERROR"],
        help="Stderr log level (default: WARNING).",
    )
    parser.add_argument("--version", action="version", version=f"digimon-training-mcp {__version__}")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(argv)
    logging.basicConfig(
        level=getattr(logging, args.log_level),
        stream=sys.stderr,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )

    runs_dir = resolve_runs_dir(args.runs_dir)
    models_dir = resolve_models_dir(args.models_dir)
    repo_root = resolve_repo_root(runs_dir, models_dir)

    print(
        f"digimon-training-mcp: ready (runs_dir={runs_dir}, models_dir={models_dir}, repo_root={repo_root})",
        file=sys.stderr,
        flush=True,
    )

    ctx = ServerContext(runs_dir=runs_dir, models_dir=models_dir, repo_root=repo_root)
    try:
        asyncio.run(run_stdio(ctx))
    except KeyboardInterrupt:
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
