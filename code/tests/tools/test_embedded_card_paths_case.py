"""Every `include_str!`/`include_bytes!` of a card YAML from the engine source must
name the file with its exact on-disk case.

Windows resolves `cards/LM/LM-027.yaml` against the real `cards/lm/` directory, so a
mismatch compiles locally and fails only on the case-sensitive CI runners (the
clone-safety job was red for two days on exactly this). Compare path components
against directory listings instead of `os.path.exists`.
"""
from __future__ import annotations

import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SRC = ROOT / "code" / "digimon-engine" / "src"
INCLUDE = re.compile(r'include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)')


def _exact_case_exists(base: Path, rel: str) -> bool:
    cur = base
    for part in Path(rel).parts:
        if part == "..":
            cur = cur.parent
            continue
        if part == ".":
            continue
        try:
            entries = os.listdir(cur)
        except OSError:
            return False
        if part not in entries:
            return False
        cur = cur / part
    return True


def test_embedded_card_paths_match_on_disk_case():
    bad = []
    for rs in SRC.rglob("*.rs"):
        text = rs.read_text(encoding="utf-8", errors="ignore")
        for m in INCLUDE.finditer(text):
            rel = m.group(1)
            if "cards/" not in rel.replace("\\", "/"):
                continue
            if not _exact_case_exists(rs.parent, rel):
                bad.append(f"{rs.relative_to(ROOT)}: include of {rel!r} does not match the on-disk case")
    assert not bad, "\n".join(bad)
