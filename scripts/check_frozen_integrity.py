#!/usr/bin/env python3
"""CI guard to prevent unsanctioned frozen script modifications."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPTS_ROOT = ROOT / "digimon_gym" / "engine" / "data" / "scripts"
MANIFEST_PATH = SCRIPTS_ROOT / "_frozen_manifest.json"
SCRIPT_FILENAME_RE = re.compile(r"^[a-z0-9]+_[0-9]{3}(?:_token)?\.py$")


def sha256(path: Path) -> str:
    # Normalize CRLF -> LF so hashes match across platforms
    content = path.read_bytes().replace(b"\r\n", b"\n")
    return hashlib.sha256(content).hexdigest()


def iter_frozen_files() -> list[Path]:
    files = []
    for set_dir in SCRIPTS_ROOT.iterdir():
        if not set_dir.is_dir() or set_dir.name in {"generated", "__pycache__"}:
            continue
        for py_file in set_dir.glob("*.py"):
            if py_file.name == "__init__.py":
                continue
            if not SCRIPT_FILENAME_RE.match(py_file.name):
                continue
            files.append(py_file)
    return sorted(files)


def main() -> int:
    if not MANIFEST_PATH.exists():
        print(f"Missing manifest: {MANIFEST_PATH}")
        return 1
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    cards = manifest.get("cards", {})
    errors = []

    tracked_relpaths = set()
    for card_id, entry in cards.items():
        relpath = entry.get("frozen_relpath")
        expected_hash = entry.get("frozen_hash")
        if not relpath or not expected_hash:
            errors.append(f"{card_id}: missing frozen_relpath/frozen_hash in manifest")
            continue
        path = SCRIPTS_ROOT / relpath
        tracked_relpaths.add(relpath.replace("\\", "/"))
        if not path.exists():
            errors.append(f"{card_id}: missing frozen file {relpath}")
            continue
        actual = sha256(path)
        if actual != expected_hash:
            errors.append(f"{card_id}: hash mismatch for {relpath}")

    for frozen_file in iter_frozen_files():
        rel = str(frozen_file.relative_to(SCRIPTS_ROOT)).replace("\\", "/")
        if rel not in tracked_relpaths:
            errors.append(f"Untracked frozen file: {rel}")

    if errors:
        print("Frozen integrity check failed:")
        for err in errors:
            print(f" - {err}")
        return 1

    print("Frozen integrity check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

