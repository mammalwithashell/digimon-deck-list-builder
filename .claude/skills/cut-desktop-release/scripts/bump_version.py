#!/usr/bin/env python3
"""Bump the desktop app version across the three files that must stay in sync.

Usage:
    python .claude/skills/cut-desktop-release/scripts/bump_version.py X.Y.Z[-suffix]

Updates (relative to the git repo root, resolved via `git rev-parse`):
  - code/src-tauri/tauri.conf.json   ->  top-level "version"
  - code/src-tauri/Cargo.toml        ->  [package] version
  - Cargo.lock                       ->  the `digimon-tcg` [[package]] version

The `Cargo.lock` edit is the error-prone one (the file has many `version = "..."`
lines); this targets only the `digimon-tcg` package block. Prints old -> new per
file and exits non-zero if any file wasn't changed, so a silent miss can't ship a
half-bumped release.
"""
import re
import subprocess
import sys
from pathlib import Path


def repo_root() -> Path:
    out = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True)
    return Path(out.strip())


def bump_tauri_conf(path: Path, new: str) -> str:
    text = path.read_text(encoding="utf-8")
    # Top-level "version": "X" — the first such key in tauri.conf.json.
    m = re.search(r'("version"\s*:\s*")([^"]+)(")', text)
    if not m:
        raise SystemExit(f"!! no \"version\" key found in {path}")
    old = m.group(2)
    text = text[: m.start()] + f'{m.group(1)}{new}{m.group(3)}' + text[m.end():]
    path.write_text(text, encoding="utf-8")
    return old


def bump_cargo_toml(path: Path, new: str) -> str:
    text = path.read_text(encoding="utf-8")
    # The [package] version — first `version = "X"` at line start.
    m = re.search(r'(?m)^(version\s*=\s*")([^"]+)(")', text)
    if not m:
        raise SystemExit(f"!! no package version found in {path}")
    old = m.group(2)
    text = text[: m.start()] + f'{m.group(1)}{new}{m.group(3)}' + text[m.end():]
    path.write_text(text, encoding="utf-8")
    return old


def bump_cargo_lock(path: Path, new: str, pkg: str = "digimon-tcg") -> str:
    text = path.read_text(encoding="utf-8")
    # Match the version line inside the `name = "<pkg>"` package block only.
    pat = re.compile(
        r'(name\s*=\s*"' + re.escape(pkg) + r'"\s*\nversion\s*=\s*")([^"]+)(")'
    )
    m = pat.search(text)
    if not m:
        raise SystemExit(f"!! no [[package]] {pkg} version found in {path}")
    old = m.group(2)
    text = text[: m.start()] + f'{m.group(1)}{new}{m.group(3)}' + text[m.end():]
    path.write_text(text, encoding="utf-8")
    return old


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: bump_version.py X.Y.Z[-suffix]")
    new = sys.argv[1].lstrip("v")
    if not re.match(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.\-]+)?$", new):
        raise SystemExit(f"!! '{new}' is not a SemVer version (expected X.Y.Z[-suffix])")

    root = repo_root()
    targets = [
        ("code/src-tauri/tauri.conf.json", bump_tauri_conf),
        ("code/src-tauri/Cargo.toml", bump_cargo_toml),
        ("Cargo.lock", bump_cargo_lock),
    ]
    for rel, fn in targets:
        p = root / rel
        if not p.exists():
            raise SystemExit(f"!! missing {p}")
        old = fn(p, new)
        status = "unchanged (already at target)" if old == new else f"{old} -> {new}"
        print(f"  {rel}: {status}")
    print(f"\nBumped to {new}. Review `git diff`, then commit + tag per the skill.")


if __name__ == "__main__":
    main()
