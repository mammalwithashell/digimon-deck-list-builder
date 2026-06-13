"""CLI entrypoint: ``python -m digimon_scenario_mcp``.

Resolves the `qa/scenarios/` + `code/frontend/e2e/` roots (ancestor-walk),
the browser API base URL, and the desktop bridge URL (auto-discovered), then
starts the MCP stdio server.
"""

from __future__ import annotations

import argparse
import asyncio
import logging
import sys
from pathlib import Path

from . import __version__
from . import fixtures as _fx
from . import spec_gen as _spec
from .server import ServerContext, run_stdio


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="digimon-scenario-mcp",
        description="WRITE-capable dev/test MCP for staging + snapshotting Digimon game states.",
    )
    parser.add_argument("--scenarios-dir", type=Path, default=None, help="Override qa/scenarios/.")
    parser.add_argument("--e2e-dir", type=Path, default=None, help="Override code/frontend/e2e/.")
    parser.add_argument(
        "--browser-url",
        default="http://127.0.0.1:8000",
        help="Hosted-API base URL for the browser target (default: http://127.0.0.1:8000).",
    )
    parser.add_argument(
        "--desktop-url",
        default=None,
        help="Tauri bridge base URL (default: auto-discover from the bridge dotfile / env).",
    )
    parser.add_argument(
        "--log-level", default="WARNING", choices=["DEBUG", "INFO", "WARNING", "ERROR"]
    )
    parser.add_argument("--version", action="version", version=f"digimon-scenario-mcp {__version__}")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = _parse_args(argv)
    logging.basicConfig(
        level=getattr(logging, args.log_level),
        stream=sys.stderr,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )

    scenarios_dir = _fx.resolve_scenarios_dir(args.scenarios_dir)
    e2e_dir = _spec.resolve_e2e_dir(args.e2e_dir)

    print(
        f"digimon-scenario-mcp: ready (scenarios_dir={scenarios_dir}, e2e_dir={e2e_dir}, "
        f"browser={args.browser_url}, desktop={args.desktop_url or 'auto'})",
        file=sys.stderr,
        flush=True,
    )

    ctx = ServerContext(
        scenarios_dir=scenarios_dir,
        e2e_dir=e2e_dir,
        browser_base_url=args.browser_url,
        desktop_base_url=args.desktop_url,
    )
    try:
        asyncio.run(run_stdio(ctx))
    except KeyboardInterrupt:
        return 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
