#!/usr/bin/env python3
"""SessionStart hook: inject a compact Digimon TCG keyword-semantics baseline plus
a pointer to deeper resources, so the assistant has light rules awareness from turn
one. Reads the committed docs/digimon-rules/keyword-semantics.md (single source of
truth) and prints it with a short banner. Silent if absent. No Pinecone / network.

This is intentionally the *compact* table only — not the full digest. The deep
digest loads on demand via the /digimon-rules skill (`deep` mode).
"""
from __future__ import annotations

import sys
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def main() -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    try:
        text = (repo_root() / "docs/digimon-rules/keyword-semantics.md").read_text(encoding="utf-8")
    except Exception:
        return 0  # artifact not present (e.g. older branch) -> stay silent
    print(
        "[digimon-rules] Baseline Digimon TCG rules awareness (compact keyword table below). "
        "For a specific rule/keyword invoke `/digimon-rules <query>` (reads the exact PDF pages); "
        "to load the full deep digest and act as a TCG thinking partner invoke `/digimon-rules deep`. "
        "Authoritative PDFs resolve under the base repo's `Digimon TCG resources/` (rule 29-style base path)."
    )
    print()
    print(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
