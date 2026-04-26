#!/usr/bin/env python3
"""Bootstrap frozen manifest from current production script folders."""

from __future__ import annotations

from pathlib import Path
import sys

PROJECT_ROOT = Path(__file__).resolve().parents[1]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from engine_py_legacy.engine.data.script_promotion import bootstrap_manifest_from_frozen, frozen_manifest_path


def main() -> None:
    manifest = bootstrap_manifest_from_frozen()
    print(f"Manifest written to: {frozen_manifest_path()}")
    print(f"Version: {manifest.get('version')}")
    print(f"Cards tracked: {len(manifest.get('cards', {}))}")


if __name__ == "__main__":
    main()
