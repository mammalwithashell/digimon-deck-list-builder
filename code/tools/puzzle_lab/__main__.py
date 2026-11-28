"""Module entrypoint: `PYTHONPATH=code python -m tools.puzzle_lab <subcommand>`."""

from __future__ import annotations

import sys

from .cli import main

if __name__ == "__main__":
    sys.exit(main())
