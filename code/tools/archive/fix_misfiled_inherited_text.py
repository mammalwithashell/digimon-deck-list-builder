#!/usr/bin/env python3
"""One-time migration: file misingested "Inherited Effect ..." text correctly.

The digimoncard.io API returns some cards' inherited effect inside
`main_effect` (prefixed with the printed "Inherited Effect" label) with
`source_effect` empty — ~431 cards across older sets (eggs/rookies like
ST6-03 Gabumon). `ingest_cards.py` filed that verbatim, so the engine's
`CardData.inherited_text` was empty and the stack inspector showed no
inherited effect for those cards. `tools.ingest_cards` now normalizes this
on ingest (`normalize_misfiled_inherited`); this script applies the same
transform to the existing data/cards.json.

Usage: python code/tools/archive/fix_misfiled_inherited_text.py
"""
import json
import os
import sys

_TOOLS_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_CODE_DIR = os.path.dirname(_TOOLS_DIR)
for p in (_CODE_DIR, _TOOLS_DIR):
    if p not in sys.path:
        sys.path.insert(0, p)

from data_paths import CARDS_JSON  # noqa: E402
from ingest_cards import normalize_misfiled_inherited  # noqa: E402


def main() -> None:
    path = str(CARDS_JSON)
    with open(path, "r", encoding="utf-8") as f:
        cards = json.load(f)

    moved = [
        cid
        for cid, card in cards.items()
        if isinstance(card, dict) and normalize_misfiled_inherited(card)
    ]

    with open(path, "w", encoding="utf-8") as f:
        json.dump(cards, f, indent=2, ensure_ascii=False)

    print(f"moved inherited text on {len(moved)} cards")
    if moved:
        print("first/last:", moved[0], "...", moved[-1])


if __name__ == "__main__":
    main()
