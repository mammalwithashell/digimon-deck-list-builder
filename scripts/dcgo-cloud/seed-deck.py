#!/usr/bin/env python3
"""Create the saved starter deck needed by the recorder's bot menu automation."""
import json
import os
from collections import Counter
from pathlib import Path

repo = Path(__file__).resolve().parents[2]
root = Path(os.environ.get("DCGO_ROOT", "/workspace/dcgo-cloud"))
cards = json.loads((repo / "data/cards.json").read_text())
book = json.loads((repo / "data/starter_decks.json").read_text())
deck = next(d for d in book["starter_decks"] if d["id"] == "starter_st1_gaia_red")
destination = root / "client/DCGO_Standalone/Assets/Decks/Cloud_ST1.txt"
destination.parent.mkdir(parents=True, exist_ok=True)
if destination.exists():
    raise SystemExit(f"Already exists; preserving {destination}")
counts = Counter(deck["egg_deck"] + deck["main_deck"])
lines = ["Name: Cloud ST1 Gaia Red", f"Key Card: {cards['ST1-15']['index']}",
         "Sort Index: 0", "", "// DeckList", ""]
lines += [f"{count} {cards[card]['card_name_eng']}   {card}" for card, count in counts.items()]
destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(destination)
